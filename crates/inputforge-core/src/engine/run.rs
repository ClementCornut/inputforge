// Rust guideline compliant 2026-03-06

//! Engine main loop and per-frame tick logic.
//!
//! The [`Engine::run`] method drives the main loop, calling
//! [`Engine::tick`] each iteration. Separating the single-frame
//! logic into `tick` makes unit testing straightforward without
//! dealing with the loop or sleep.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::action::{Action, Mapping};
use crate::callbacks::ReleaseCallback;
use crate::device::traits::HotplugEvent;
use crate::error::Result;
use crate::pipeline::{self, OutputOwnerScope, PipelineContext, PipelineOutput};
use crate::profile::library::{
    add_external_profile_to_library, duplicate_library_profile, rename_library_profile,
};
use crate::profile::manager::{
    create_profile_in, delete_profile, list_profiles_in, sanitize_filename,
};
use crate::profile::{CalibrationEntry, Profile};
use crate::snapshot::pending_delete::{
    PENDING_SUBDIR, list_visible, purge_expired_pending_deletes, resolve_snapshot_namespace,
    stage_delete_in, undo_delete_by_id,
};
use crate::state::{
    ActiveSnapshotRow, AppState, DeviceCalibrationStore, DeviceState, EngineStatus,
    InputCacheEntry, ProfileLibraryRow, ProfileOrigin,
};
use crate::types::{DeviceDiagnostics, DeviceInfo, InputAddress, InputEvent, InputId, InputValue};

use super::Engine;
use super::command::EngineCommand;
use super::dependencies::active_mappings_for_event;
use super::output_handler::{
    dispatch_output_action, process_pipeline_outputs, record_outputs_to_cache,
    refresh_axes_for_mode_change,
};
use super::output_state::OwnerScopeKey;

/// Target poll interval for the engine loop.
///
/// 1 ms provides responsive input handling at ~1000 Hz without
/// excessive CPU usage. The OS scheduler may add jitter.
const POLL_INTERVAL: Duration = Duration::from_millis(1);

/// Retention window for snapshot pending-delete manifests.
///
/// Manifests older than this are purged on every profile load. Seven
/// days keeps the user's undo affordance alive across a typical week-long
/// pause without indefinitely keeping deleted snapshot bytes on disk.
const PENDING_DELETE_RETENTION_DAYS: i64 = 7;

/// Maximum mode-name length, measured in extended grapheme clusters
/// (UAX #29 extended).
///
/// Re-exported from `crate::engine` and consumed by the GUI inline
/// editors so a single source of truth governs both the engine's
/// out-of-band-caller rejection and the inline UX feedback. Drift is
/// impossible because the GUI imports this constant directly rather
/// than mirroring it.
pub const MAX_MODE_NAME_GRAPHEMES: usize = 64;

/// Returns `Err(InvalidConfig)` if `name` is empty or exceeds the
/// grapheme-cluster cap. Shared by `AddMode`, `RenameMode`, and
/// `SetDefaultMode` so the three handlers enforce identical policy.
fn validate_mode_name_for_engine(
    name: &str,
    empty_reason: &str,
) -> std::result::Result<(), crate::error::EngineError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(crate::error::EngineError::InvalidConfig {
            reason: empty_reason.to_owned(),
        });
    }
    let grapheme_count =
        unicode_segmentation::UnicodeSegmentation::graphemes(trimmed, true).count();
    if grapheme_count > MAX_MODE_NAME_GRAPHEMES {
        return Err(crate::error::EngineError::InvalidConfig {
            reason: format!(
                "mode name too long ({grapheme_count} graphemes, max {MAX_MODE_NAME_GRAPHEMES})"
            ),
        });
    }
    Ok(())
}

/// Read mode count and last-edit timestamp for the profile file at `path`.
///
/// Best-effort: any I/O or parse failure surfaces as a warning log and
/// produces `(0, None)` so the projection always has a row to show.
/// Used by `refresh_profile_library_rows` and never by command paths.
fn read_profile_metadata(path: &Path) -> (u32, Option<DateTime<Utc>>) {
    let mode_count = match Profile::load(path) {
        Ok(profile) => {
            // The mode count is bounded by profile validation elsewhere in
            // the engine; saturating to u32::MAX keeps the cast lossless
            // for any plausible mode-list size.
            u32::try_from(profile.modes().len()).unwrap_or(u32::MAX)
        }
        Err(e) => {
            tracing::warn!(
                target: "engine",
                profile_path = %path.display(),
                error = %e,
                "engine.profile_library.load_failure"
            );
            0
        }
    };

    let last_edited_at = match std::fs::metadata(path).and_then(|meta| meta.modified()) {
        Ok(modified) => Some(DateTime::<Utc>::from(modified)),
        Err(e) => {
            tracing::warn!(
                target: "engine",
                profile_path = %path.display(),
                error = %e,
                "engine.profile_library.mtime_failure"
            );
            None
        }
    };

    (mode_count, last_edited_at)
}

impl Engine {
    /// Run the main engine loop until shutdown.
    ///
    /// Blocks the current thread. Call from a dedicated engine thread.
    /// The loop processes commands, polls input, and writes output
    /// each iteration, sleeping for [`POLL_INTERVAL`] between frames.
    ///
    /// # Errors
    ///
    /// Returns an error if a critical I/O operation fails mid-loop.
    pub fn run(&mut self) -> Result<()> {
        while !self.shutdown {
            self.tick()?;
            std::thread::sleep(POLL_INTERVAL);
        }
        Ok(())
    }

    /// Execute a single engine frame.
    ///
    /// Processes pending commands, polls input, routes events through direct
    /// active-mode mapping lookup and pipelines, and writes output. Call this
    /// directly in tests instead of [`run`](Self::run) to avoid the loop and
    /// sleep.
    ///
    /// # Errors
    ///
    /// Returns an error if output writing fails.
    #[expect(
        clippy::too_many_lines,
        reason = "single-frame logic is intentionally co-located for readability; \
                  splitting into sub-functions would obscure the event-processing flow"
    )]
    pub fn tick(&mut self) -> Result<()> {
        self.process_commands()?;

        // Always poll input and handle hotplug events so devices and
        // live input values are visible in the GUI even when stopped.
        self.event_buffer.clear();
        self.input.poll(&mut self.event_buffer);

        let hotplug_events = self.input.hotplug_events();
        if !hotplug_events.is_empty() {
            self.handle_hotplug(&hotplug_events);
        }

        // Update input cache from all events regardless of engine status.
        // The GUI reads the cache to display live axis/button values.
        if !self.event_buffer.is_empty() {
            let mut state = self.state.write();
            for event in &self.event_buffer {
                state.input_cache.update(&event.source, &event.value);
            }
        }

        if self.read_status() != EngineStatus::Running {
            return Ok(());
        }

        // Get profile data needed for this frame.
        // Clone mappings + mode list to avoid holding the lock during processing.
        let (profile_name, mappings, mode_list) = {
            let state = self.state.read();
            match &state.active_profile {
                Some(profile) => (
                    profile.name().to_owned(),
                    profile.mappings().to_vec(),
                    profile.modes().clone(),
                ),
                None => return Ok(()),
            }
        };

        // On first tick after activation, refresh all cached axis outputs so
        // vJoy reflects current physical device positions without waiting for
        // a new input event.
        self.apply_activation_refresh(&mappings)?;

        // Process each input event.
        // Move the buffer out of self so the loop body can borrow other
        // &mut self fields (state, mode_state, callbacks). After the loop
        // the cleared buffer is restored to reuse its heap allocation.
        let mut events = std::mem::take(&mut self.event_buffer);
        self.output_buffer.clear();
        for event in &events {
            // Update the input cache.
            let mut state = self.state.write();
            state.input_cache.update(&event.source, &event.value);
            drop(state);

            // Fire release callbacks BEFORE resolving mappings.
            //
            // This ordering intentionally differs from the design plan (which
            // places callbacks after output). For temporary mode pops it is
            // more correct: when the user releases a "shift" button, the pop
            // must happen first so the release event's mapping is resolved in
            // the restored mode, not the temporary one.
            let mut callbacks_changed_mode = false;
            if let InputValue::Button { pressed: false } = &event.value {
                let mode_before_callbacks = self.mode_state.current().to_owned();
                let callbacks = self.callbacks.fire(&event.source);
                for callback in callbacks {
                    match callback {
                        ReleaseCallback::PopTemporaryMode => {
                            self.mode_state.pop_temporary();
                        }
                        ReleaseCallback::Custom(f) => f(),
                    }
                }
                callbacks_changed_mode = self.mode_state.current() != mode_before_callbacks;
            }

            let active_mappings =
                active_mappings_for_event(&mappings, &event.source, self.mode_state.current());
            if active_mappings.is_empty() {
                continue;
            }

            for mapping in active_mappings {
                // Single lock acquisition for calibration lookup and pipeline context.
                let guard = self.state.read();
                let Some((current_value, input_value)) =
                    pipeline_input_for_mapping(mapping, event, &guard)
                else {
                    continue;
                };

                let mut ctx = PipelineContext {
                    current_value,
                    input_value,
                    outputs: Vec::new(),
                    input_cache: &guard.input_cache,
                };
                pipeline::execute_pipeline_with_scope(
                    &mapping.actions,
                    &mut ctx,
                    OutputOwnerScope::new(
                        profile_name.clone(),
                        mapping.mode.clone(),
                        mapping.input.clone(),
                    ),
                );
                let outputs = std::mem::take(&mut ctx.outputs);
                drop(guard);

                let current_owners = outputs
                    .iter()
                    .filter_map(|output| match output {
                        PipelineOutput::Keyboard { owner, .. }
                        | PipelineOutput::Mouse { owner, .. } => Some(owner.clone()),
                        PipelineOutput::SetAxis { .. }
                        | PipelineOutput::SetButton { .. }
                        | PipelineOutput::ChangeMode { .. } => None,
                    })
                    .collect::<Vec<_>>();
                let owner_scope = current_owners.first().map_or_else(
                    || {
                        OwnerScopeKey::new(
                            profile_name.clone(),
                            mapping.mode.clone(),
                            mapping.input.clone(),
                        )
                    },
                    OwnerScopeKey::from_owner,
                );

                // Process pipeline outputs.
                let result = process_pipeline_outputs(
                    &outputs,
                    self.output.as_mut(),
                    self.keyboard.as_mut(),
                    self.mouse.as_mut(),
                    &mut self.output_state,
                    &mut self.mode_state,
                    &mode_list,
                    &mut self.callbacks,
                    &event.source,
                )?;
                for action in self
                    .output_state
                    .reconcile_absent_owners_for_scope(&owner_scope, &current_owners)
                {
                    dispatch_output_action(
                        action,
                        &mut self.output_state,
                        self.keyboard.as_mut(),
                        self.mouse.as_mut(),
                    )?;
                }

                self.output_buffer.extend_from_slice(&outputs);

                // If mode changed (via pipeline output or release callbacks),
                // refresh all cached axes through the new mode.
                if result.mode_changed || callbacks_changed_mode {
                    let mut guard = self.state.write();
                    let state: &mut AppState = &mut guard;
                    refresh_axes_for_mode_change(
                        &state.input_cache,
                        &mappings,
                        self.mode_state.current(),
                        self.output.as_mut(),
                        &mut state.output_cache,
                    )?;
                    self.output_buffer.clear();
                    break;
                }
            }
        }

        // Restore the buffer to reuse its heap allocation next frame.
        events.clear();
        self.event_buffer = events;

        // Flush pipeline outputs to the output cache in a single write lock.
        if !self.output_buffer.is_empty() {
            let mut state = self.state.write();
            record_outputs_to_cache(&self.output_buffer, &mut state.output_cache);
        }

        // Flush output sink.
        self.output.flush()?;

        // Write current mode to shared state.
        let mut state = self.state.write();
        self.mode_state
            .current()
            .clone_into(&mut state.current_mode);
        drop(state);

        Ok(())
    }

    /// Refresh all cached axis outputs if an activation refresh is pending.
    ///
    /// Consumes the `pending_output_refresh` flag and runs
    /// [`refresh_axes_for_mode_change`] so vJoy reflects current physical
    /// device positions on the first tick after activation.
    fn apply_activation_refresh(&mut self, mappings: &[Mapping]) -> Result<()> {
        if !self.pending_output_refresh {
            return Ok(());
        }
        self.pending_output_refresh = false;
        let mut guard = self.state.write();
        let state: &mut AppState = &mut guard;
        refresh_axes_for_mode_change(
            &state.input_cache,
            mappings,
            self.mode_state.current(),
            self.output.as_mut(),
            &mut state.output_cache,
        )
    }

    /// Process all pending commands from the GUI.
    fn process_commands(&mut self) -> Result<()> {
        loop {
            match self.commands.try_recv() {
                Ok(cmd) => self.handle_command(cmd)?,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.shutdown = true;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Handle a single engine command.
    #[expect(
        clippy::too_many_lines,
        reason = "single match dispatch; each arm is a distinct command, \
                  splitting into sub-functions would obscure the command flow"
    )]
    pub(crate) fn handle_command(&mut self, cmd: EngineCommand) -> Result<()> {
        match cmd {
            EngineCommand::LoadProfile(path) => {
                self.purge_all_namespaces();
                self.reload_profile_from_disk(&path)?;
                let origin = self.profile_origin_for_path(&path);
                {
                    let mut state = self.state.write();
                    state.active_profile_origin = Some(origin);
                };
                if let Some((profile_path, namespace_dir)) = self.resolved_snapshot_target() {
                    let _ = crate::snapshot::create_in(
                        &profile_path,
                        &namespace_dir,
                        crate::snapshot::SnapshotKind::AutoSessionStart,
                        None,
                        &self.settings.snapshot,
                    )?;
                    let _ = crate::snapshot::prune_in(&namespace_dir, &self.settings.snapshot)?;
                }
                self.refresh_profile_library_rows()?;
                self.refresh_active_snapshot_rows()?;
                self.persist_last_profile()?;
            }
            EngineCommand::CreateProfile { name } => {
                let path = create_profile_in(&name, &self.profile_library_dir())?;
                self.reload_profile_from_disk(&path)?;
                self.mark_profile_loaded(ProfileOrigin::Library);
                self.refresh_profile_library_rows()?;
                self.refresh_active_snapshot_rows()?;
                self.persist_last_profile()?;
            }
            EngineCommand::LoadExternalProfileOnce(path) => {
                self.purge_all_namespaces();
                self.reload_profile_from_disk(&path)?;
                self.mark_profile_loaded(ProfileOrigin::External);
                if let Some((profile_path, namespace_dir)) = self.resolved_snapshot_target() {
                    let _ = crate::snapshot::create_in(
                        &profile_path,
                        &namespace_dir,
                        crate::snapshot::SnapshotKind::AutoSessionStart,
                        None,
                        &self.settings.snapshot,
                    )?;
                    let _ = crate::snapshot::prune_in(&namespace_dir, &self.settings.snapshot)?;
                }
                self.refresh_profile_library_rows()?;
                self.refresh_active_snapshot_rows()?;
            }
            EngineCommand::AddExternalProfileToLibrary { path, name } => {
                let imported =
                    add_external_profile_to_library(&path, &name, &self.profile_library_dir())?;
                self.reload_profile_from_disk(&imported.path)?;
                self.mark_profile_loaded(ProfileOrigin::Library);
                self.refresh_profile_library_rows()?;
                self.refresh_active_snapshot_rows()?;
                self.persist_last_profile()?;
            }
            EngineCommand::RenameProfile { old_name, new_name } => {
                let old_path = self.profile_path_for_name(&old_name);
                let was_active = self
                    .state
                    .read()
                    .profile_path
                    .as_ref()
                    .is_some_and(|path| path == &old_path);
                let renamed = rename_library_profile(&old_path, &new_name)?;
                if was_active {
                    self.reload_profile_from_disk(&renamed.path)?;
                    self.state.write().active_profile_origin = Some(ProfileOrigin::Library);
                    self.refresh_active_snapshot_rows()?;
                    self.persist_last_profile()?;
                }
                self.refresh_profile_library_rows()?;
            }
            EngineCommand::DuplicateProfile { source_path, name } => {
                let _ =
                    duplicate_library_profile(&source_path, &name, &self.profile_library_dir())?;
                self.refresh_profile_library_rows()?;
            }
            EngineCommand::DeleteProfile { name } => {
                let path = self.profile_path_for_name(&name);
                delete_profile(&path)?;
                let was_active = self
                    .state
                    .read()
                    .profile_path
                    .as_ref()
                    .is_some_and(|profile_path| profile_path == &path);
                if was_active {
                    let mut state = self.state.write();
                    state.active_profile = None;
                    state.profile_path = None;
                    state.active_profile_origin = None;
                    state.active_snapshot_rows.clear();
                    state.engine_status = EngineStatus::Stopped;
                    drop(state);
                    self.mode_state = crate::mode::ModeState::new("Default".to_owned());
                    self.callbacks.clear();
                    self.persist_last_profile()?;
                }
                self.refresh_profile_library_rows()?;
            }
            EngineCommand::RevealProfile { path } => {
                if let Err(e) = crate::profile::library::reveal_profile_in_explorer(&path) {
                    tracing::warn!(
                        target: "engine",
                        profile_path = %path.display(),
                        error = %e,
                        "engine.profile.reveal_failed"
                    );
                }
            }
            EngineCommand::Activate | EngineCommand::Resume => {
                let mut state = self.state.write();
                state.engine_status = EngineStatus::Running;
                drop(state);
                self.pending_output_refresh = true;
            }
            EngineCommand::Deactivate => {
                self.output.flush()?;
                let mut state = self.state.write();
                state.engine_status = EngineStatus::Stopped;
            }
            EngineCommand::Pause => {
                let mut state = self.state.write();
                state.engine_status = EngineStatus::Paused;
            }
            EngineCommand::SetCalibration {
                device,
                axis,
                calibration,
            } => {
                let mut state = self.state.write();
                state.calibrations.set(device, axis, calibration);
            }
            EngineCommand::SaveCalibrations => {
                self.save_calibrations_to_profile();
            }
            EngineCommand::SetMapping {
                input,
                mode,
                name,
                actions,
            } => {
                self.set_mapping(&input, &mode, name, actions);
                self.pending_output_refresh = true;
            }
            EngineCommand::ReorderMapping {
                input,
                mode,
                target_index_in_group,
            } => {
                self.reorder_mapping_in_group(&input, &mode, target_index_in_group);
            }
            EngineCommand::Shutdown => {
                self.shutdown = true;
            }
            EngineCommand::SwitchMode { mode } => {
                if self.mode_state.current() == mode {
                    return Ok(());
                }
                let modes = if let Some(p) = self.state.read().active_profile.as_ref() {
                    p.modes().clone()
                } else {
                    tracing::warn!(
                        target: "engine",
                        "SwitchMode dispatched with no profile; ignoring"
                    );
                    return Ok(());
                };
                self.mode_state.switch_to(&mode, &modes)?;
                let mut state = self.state.write();
                mode.clone_into(&mut state.current_mode);
                drop(state);
                self.pending_output_refresh = true;
                tracing::info!(target: "engine", mode = %mode, "SwitchMode applied");
            }
            EngineCommand::ReloadSettings => {
                self.settings = crate::settings::AppSettings::load_from(&self.settings_path);
                let mut state = self.state.write();
                state.snapshot_config = self.settings.snapshot.clone();
                state.startup = self.settings.startup.clone();
                drop(state);
                tracing::info!(target: "engine", "settings reloaded");
            }
            EngineCommand::SetSnapshotConfig { config } => {
                // Step 1: capture the prior config for rollback on save failure.
                let old_config = self.settings.snapshot.clone();

                // Step 2: replace in memory and persist. On save failure, restore
                // the in-memory copy so it matches the on-disk truth, push a
                // warning, and return without attempting the prune step.
                self.settings.snapshot = config.clone();
                // Mirror into AppState so the GUI projection observes the change
                // on the next polling tick. Matches the device_aliases mirror pattern.
                self.state.write().snapshot_config = self.settings.snapshot.clone();
                if let Err(e) = self.settings.save_to(&self.settings_path) {
                    tracing::warn!(
                        target: "settings",
                        error = %e,
                        "failed to persist settings.toml; rolling back in-memory snapshot config"
                    );
                    self.settings.snapshot = old_config;
                    // Revert the AppState mirror to the rolled-back value so
                    // the GUI projection does not surface a transient bogus value.
                    let mut state = self.state.write();
                    state.snapshot_config = self.settings.snapshot.clone();
                    state.warnings.push(format!("Could not save settings: {e}"));
                    drop(state);
                    return Ok(());
                }

                // Step 3: prune the active namespace when max_count decreased.
                // No-op when the count is the same or larger, or when no
                // profile is loaded.
                let mut pruned = 0_usize;
                if config.max_count < old_config.max_count
                    && let Some((_, namespace_dir)) = self.resolved_snapshot_target()
                {
                    match crate::snapshot::prune_in(&namespace_dir, &self.settings.snapshot) {
                        Ok(removed) => pruned = removed,
                        Err(e) => {
                            tracing::warn!(
                                target: "settings",
                                error = %e,
                                "settings saved but snapshot prune failed; in-memory \
                                 and on-disk settings remain consistent"
                            );
                            self.state
                                .write()
                                .warnings
                                .push(format!("Snapshot prune failed after settings save: {e}"));
                        }
                    }
                }
                self.refresh_active_snapshot_rows()?;

                tracing::info!(
                    target: "settings",
                    old_max_count = old_config.max_count,
                    new_max_count = self.settings.snapshot.max_count,
                    pruned,
                    "snapshot config updated"
                );
            }
            EngineCommand::SetDeviceAlias { device, alias } => {
                self.settings.set_device_alias(device.clone(), alias);
                self.settings.save_to(&self.settings_path)?;
                self.state
                    .write()
                    .device_aliases
                    .clone_from(&self.settings.device_aliases);
                tracing::info!(
                    target: "engine",
                    device = %device.0,
                    "device alias persisted"
                );
            }
            EngineCommand::CreateSnapshot { kind, label } => {
                let resolved = self.resolved_snapshot_target();
                if let Some((path, namespace_dir)) = resolved {
                    let _ = crate::snapshot::create_in(
                        &path,
                        &namespace_dir,
                        kind,
                        label,
                        &self.settings.snapshot,
                    )?;
                    let _ = crate::snapshot::prune_in(&namespace_dir, &self.settings.snapshot)?;
                    self.refresh_active_snapshot_rows()?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "CreateSnapshot dispatched with no profile loaded"
                    );
                }
            }
            EngineCommand::DeleteSnapshot { id } => {
                let resolved = self.resolved_snapshot_target();
                if let Some((path, namespace_dir)) = resolved {
                    let pending_dir = namespace_dir.join(PENDING_SUBDIR);
                    let _ = stage_delete_in(&path, &namespace_dir, &id, &pending_dir)?;
                    self.refresh_active_snapshot_rows()?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "DeleteSnapshot dispatched with no profile loaded"
                    );
                }
            }
            EngineCommand::PinSnapshot { id, pinned } => {
                let resolved = self.resolved_snapshot_target();
                if let Some((_, namespace_dir)) = resolved {
                    crate::snapshot::pin_in(&namespace_dir, &id, pinned)?;
                    self.refresh_active_snapshot_rows()?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "PinSnapshot dispatched with no profile loaded"
                    );
                }
            }
            EngineCommand::RenameSnapshot { id, label } => {
                let resolved = self.resolved_snapshot_target();
                if let Some((_, namespace_dir)) = resolved {
                    crate::snapshot::rename_in(&namespace_dir, &id, label)?;
                    self.refresh_active_snapshot_rows()?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "RenameSnapshot dispatched with no profile loaded"
                    );
                }
            }
            EngineCommand::AddMode { name } => {
                validate_mode_name_for_engine(&name, "mode name cannot be empty")?;
                // Bind the read-guarded snapshot in its own scope before
                // acquiring the write lock to avoid a non-reentrant deadlock
                // if the rvalue temporary's drop point ever shifts.
                let path = { self.state.read().profile_path.clone() };
                let mut state = self.state.write();
                let Some(profile) = state.active_profile.as_mut() else {
                    tracing::warn!(target: "engine", "AddMode dispatched with no profile; ignoring");
                    return Ok(());
                };
                let modes = profile.modes().with_appended(&name)?;
                profile.set_modes(modes);
                if let Some(path) = path.as_ref() {
                    profile.save(path).map_err(|e| {
                        tracing::error!(
                            target: "engine",
                            path = %path.display(),
                            error = %e,
                            "failed to persist AddMode"
                        );
                        e
                    })?;
                }
                tracing::info!(target: "engine", mode = %name, "AddMode applied");
            }
            EngineCommand::RenameMode { from, to } => {
                // Validate both names against the same policy. Without
                // this, an oversized `from` would fall through to
                // `with_renamed` and surface as `ModeNotFound`, leaking
                // an internal detail (the name doesn't match a mode)
                // when the policy reason (length cap) is what should
                // surface. Symmetric validation pins the contract.
                validate_mode_name_for_engine(&from, "source mode name cannot be empty")?;
                validate_mode_name_for_engine(&to, "mode name cannot be empty")?;
                if from == to {
                    return Ok(());
                }
                let path = { self.state.read().profile_path.clone() };
                let mut state = self.state.write();
                let Some(profile) = state.active_profile.as_mut() else {
                    tracing::warn!(target: "engine", "RenameMode dispatched with no profile; ignoring");
                    return Ok(());
                };

                // Atomicity contract, order is load-bearing:
                //   1. `with_renamed` clones the mode list and validates the
                //      collision (returns Err without mutating). Must run
                //      first so a name collision doesn't leave a partial
                //      mapping rewrite behind.
                //   2. `rename_mode_refs` mutates mappings + startup_mode
                //      in one pass. Infallible, no rollback path needed.
                //   3. `set_modes` swaps in the new list last. Single-shot,
                //      infallible, once the cascade has succeeded the
                //      list replacement cannot fail partway.
                // Reordering risks: (2)/(3) before (1) mutates mappings
                // before the list is validated against the new name, which
                // would orphan mappings on collision.
                // Step 1: list rewrite (errors on missing-from / collision).
                let new_modes = profile.modes().with_renamed(&from, &to)?;
                // Step 2: cascade across mappings + startup.
                let touched = profile.rename_mode_refs(&from, &to);
                // Step 3: swap the new list in last.
                profile.set_modes(new_modes);

                // Step 3: runtime-state cascade.
                if state.current_mode == from {
                    to.clone_into(&mut state.current_mode);
                }
                drop(state);

                self.mode_state.rename_in_place(&from, &to);

                if let Some(path) = path.as_ref() {
                    let state_read = self.state.read();
                    let Some(profile) = state_read.active_profile.as_ref() else {
                        // Profile has been unloaded between the write
                        // lock release above and this read. The mutation
                        // we just made is gone with it; nothing to save.
                        return Ok(());
                    };
                    profile.save(path).map_err(|e| {
                        tracing::error!(
                            target: "engine",
                            path = %path.display(),
                            error = %e,
                            "failed to persist RenameMode"
                        );
                        e
                    })?;
                }

                tracing::info!(
                    target: "engine",
                    from = %from,
                    to = %to,
                    mappings_touched = touched,
                    "RenameMode applied"
                );
            }
            EngineCommand::DeleteMode { name } => {
                // Validate the name first so empty / oversized inputs
                // surface as `InvalidConfig` rather than falling through
                // to `contains` and returning `ModeNotFound` (the wrong
                // error register for a policy violation).
                validate_mode_name_for_engine(&name, "mode name cannot be empty")?;
                let path = { self.state.read().profile_path.clone() };
                let mut state = self.state.write();
                let Some(profile) = state.active_profile.as_mut() else {
                    tracing::warn!(target: "engine", "DeleteMode dispatched with no profile; ignoring");
                    return Ok(());
                };

                if profile.modes().first() == name {
                    return Err(crate::error::EngineError::InvalidConfig {
                        reason: "cannot delete first mode".to_owned(),
                    });
                }
                if !profile.modes().contains(&name) {
                    return Err(crate::error::EngineError::ModeNotFound { name: name.clone() });
                }

                let startup = profile.settings().startup_mode().to_owned();
                if startup == name {
                    return Err(crate::error::EngineError::InvalidConfig {
                        reason: format!("cannot delete startup mode '{startup}'"),
                    });
                }

                let new_modes = profile.modes().with_removed(&name)?;
                profile.set_modes(new_modes);
                let mappings_dropped = profile.remove_mappings_for_mode(&name);

                // Runtime state cascade.
                if state.current_mode == name {
                    startup.clone_into(&mut state.current_mode);
                }
                drop(state);

                // ModeState reset.
                if self.mode_state.current() == name {
                    let modes = self
                        .state
                        .read()
                        .active_profile
                        .as_ref()
                        .map(|profile| profile.modes().clone());
                    if let Some(modes) = modes {
                        self.mode_state.switch_to(&startup, &modes)?;
                    }
                }
                self.mode_state
                    .clear_stack_entries(std::slice::from_ref(&name));

                if let Some(path) = path.as_ref() {
                    let state_read = self.state.read();
                    let Some(profile) = state_read.active_profile.as_ref() else {
                        return Ok(());
                    };
                    profile.save(path).map_err(|e| {
                        tracing::error!(
                            target: "engine",
                            path = %path.display(),
                            error = %e,
                            "failed to persist DeleteMode"
                        );
                        e
                    })?;
                }

                tracing::info!(
                    target: "engine",
                    mode = %name,
                    mappings_dropped,
                    "DeleteMode applied"
                );
            }
            EngineCommand::SetDefaultMode { name } => {
                validate_mode_name_for_engine(&name, "startup mode name cannot be empty")?;
                let path = { self.state.read().profile_path.clone() };
                let mut state = self.state.write();
                let Some(profile) = state.active_profile.as_mut() else {
                    tracing::warn!(target: "engine", "SetDefaultMode dispatched with no profile; ignoring");
                    return Ok(());
                };
                if !profile.modes().contains(&name) {
                    return Err(crate::error::EngineError::ModeNotFound { name: name.clone() });
                }
                profile.set_startup_mode(name.clone());

                if let Some(path) = path.as_ref() {
                    profile.save(path).map_err(|e| {
                        tracing::error!(
                            target: "engine",
                            path = %path.display(),
                            error = %e,
                            "failed to persist SetDefaultMode"
                        );
                        e
                    })?;
                }
                tracing::info!(target: "engine", mode = %name, "SetDefaultMode applied");
            }
            EngineCommand::RestoreSnapshot { id } => {
                let Some((path, namespace_dir)) = self.resolved_snapshot_target() else {
                    tracing::warn!(target: "snapshot", "RestoreSnapshot dispatched with no profile loaded");
                    return Ok(());
                };

                // Step 1, capture AutoBeforeRestore (always fires; never deduped).
                let auto = crate::snapshot::create_in(
                    &path,
                    &namespace_dir,
                    crate::snapshot::SnapshotKind::AutoBeforeRestore,
                    None,
                    &self.settings.snapshot,
                )?;
                let _ = crate::snapshot::prune_in(&namespace_dir, &self.settings.snapshot)?;

                // Step 2, strip meta + atomically write target body to live path.
                crate::snapshot::restore_in(&path, &namespace_dir, &id)?;

                // Step 3, reload from disk; auto-rollback on failure.
                if let Err(reload_err) = self.reload_profile_from_disk(&path) {
                    tracing::error!(
                        target: "snapshot",
                        ?reload_err,
                        "restore reload failed; rolling back to AutoBeforeRestore"
                    );
                    if let Some(auto_snap) = auto {
                        crate::snapshot::restore_in(&path, &namespace_dir, &auto_snap.id)?;
                        self.reload_profile_from_disk(&path)?;
                    }
                    return Err(reload_err);
                }

                self.refresh_active_snapshot_rows()?;

                tracing::info!(
                    target: "snapshot",
                    id = %id,
                    "RestoreSnapshot complete"
                );
            }
            EngineCommand::UndoSnapshotDelete { id } => {
                let resolved = self.resolved_snapshot_target();
                if let Some((_, namespace_dir)) = resolved {
                    let pending_dir = namespace_dir.join(PENDING_SUBDIR);
                    undo_delete_by_id(&pending_dir, &id)?;
                    self.refresh_active_snapshot_rows()?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "UndoSnapshotDelete dispatched with no profile loaded"
                    );
                }
            }

            EngineCommand::RemoveMapping { input, mode } => {
                self.remove_mapping(&input, &mode);
                self.pending_output_refresh = true;
            }
            EngineCommand::SetMappingsBulk {
                entries,
                snapshot_label,
            } => {
                self.set_mappings_bulk(&entries, snapshot_label);
                self.pending_output_refresh = true;
            }
            EngineCommand::SetAutostart { enabled } => {
                // Step 1: compute argv from the persisted start-minimized
                // setting; auto-launch is dumb about that flag.
                let owned_args: Vec<&str> = if self.settings.startup.start_minimized_to_tray {
                    vec!["--start-minimized"]
                } else {
                    vec![]
                };

                // Step 2: OS write first; on failure, do NOT touch settings or
                // AppState (the mirror chain plus polling will resync the UI).
                if let Err(e) = self.autostart.set_enabled(enabled, &owned_args) {
                    tracing::warn!(
                        target: "autostart",
                        %e,
                        enabled,
                        "autostart OS write failed"
                    );
                    self.state
                        .write()
                        .warnings
                        .push("Could not change launch-at-startup setting.".to_owned());
                    return Ok(());
                }

                // Step 3: persist + mirror; on save failure, roll back both.
                let prior = self.settings.startup.clone();
                self.settings.startup.launch_at_startup = enabled;
                self.state.write().startup = self.settings.startup.clone();
                if let Err(e) = self.settings.save_to(&self.settings_path) {
                    tracing::warn!(
                        target: "settings",
                        error = %e,
                        "failed to persist settings.toml; rolling back in-memory startup"
                    );
                    self.settings.startup = prior;
                    let mut state = self.state.write();
                    state.startup = self.settings.startup.clone();
                    state.warnings.push(format!("Could not save settings: {e}"));
                    return Ok(());
                }

                tracing::info!(
                    target: "engine",
                    launch_at_startup = self.settings.startup.launch_at_startup,
                    "autostart updated"
                );
            }
            EngineCommand::SetStartMinimizedToTray { enabled } => {
                // Step 1: capture prior for rollback on save failure.
                let prior = self.settings.startup.clone();

                // Step 2: persist + mirror.
                self.settings.startup.start_minimized_to_tray = enabled;
                self.state.write().startup = self.settings.startup.clone();
                if let Err(e) = self.settings.save_to(&self.settings_path) {
                    tracing::warn!(
                        target: "settings",
                        error = %e,
                        "failed to persist settings.toml; rolling back in-memory startup"
                    );
                    self.settings.startup = prior;
                    let mut state = self.state.write();
                    state.startup = self.settings.startup.clone();
                    state.warnings.push(format!("Could not save settings: {e}"));
                    return Ok(());
                }

                // Step 3: best-effort autostart argv re-register when on.
                if self.settings.startup.launch_at_startup {
                    let owned_args: Vec<&str> = if self.settings.startup.start_minimized_to_tray {
                        vec!["--start-minimized"]
                    } else {
                        vec![]
                    };
                    if let Err(e) = self.autostart.set_enabled(true, &owned_args) {
                        tracing::warn!(
                            target: "autostart",
                            %e,
                            "could not refresh autostart argv after start-minimized toggle"
                        );
                        self.state.write().warnings.push(
                            "Saved, but could not update the auto-launch arguments. \
                             Restart of InputForge may use the previous setting."
                                .to_owned(),
                        );
                    }
                }

                tracing::info!(
                    target: "engine",
                    start_minimized_to_tray = self.settings.startup.start_minimized_to_tray,
                    "start-minimized preference updated"
                );
            }
        }
        Ok(())
    }

    /// Reload the active profile from disk and rebuild dependent in-memory state.
    ///
    /// Resets calibrations, mode state, callbacks, and the active profile to
    /// match `path` on disk. Shared between `LoadProfile` and `RestoreSnapshot`.
    ///
    /// # Errors
    ///
    /// Returns an error if the profile file cannot be read or parsed.
    fn reload_profile_from_disk(&mut self, path: &Path) -> Result<()> {
        let profile = Profile::load(path)?;
        let startup_mode = profile.settings().startup_mode().to_owned();
        self.mode_state = crate::mode::ModeState::new(startup_mode.clone());
        self.callbacks.clear();

        let mut state = self.state.write();

        // Load calibrations from profile.
        state.calibrations = DeviceCalibrationStore::new();
        for entry in profile.calibrations() {
            match entry.to_calibration() {
                Ok(cal) => {
                    state
                        .calibrations
                        .set(entry.device.clone(), entry.axis, cal);
                }
                Err(e) => {
                    tracing::warn!(
                        device = %entry.device.0,
                        axis = entry.axis,
                        error = %e,
                        "skipping invalid calibration entry"
                    );
                }
            }
        }

        state.active_profile = Some(profile);
        state.profile_path = Some(path.to_path_buf());
        state.current_mode = startup_mode;
        Ok(())
    }

    fn profile_library_dir(&self) -> PathBuf {
        if self.settings_path.as_os_str().is_empty() {
            return crate::settings::AppSettings::profiles_dir();
        }
        self.settings_path
            .parent()
            .map_or_else(crate::settings::AppSettings::profiles_dir, |dir| {
                dir.join("profiles")
            })
    }

    fn mark_profile_loaded(&self, origin: ProfileOrigin) {
        let mut state = self.state.write();
        state.active_profile_origin = Some(origin);
        state.engine_status = EngineStatus::Stopped;
    }

    /// Mirror `state.profile_path` into `settings.last_profile` and
    /// persist to `settings_path`. Called from every profile-lifecycle
    /// command that changes which profile is active, so a fresh launch
    /// reopens whichever profile the user had last. `LoadExternalProfileOnce`
    /// deliberately skips this: the "Once" semantic means transient by
    /// design (the `OpenChoice` dialog's two branches are "Add to library"
    /// for sticky import vs "Load once" for one-shot use).
    ///
    /// Test harnesses that don't care about settings persistence pass an
    /// empty `settings_path` (`PathBuf::new()`); mirror the same
    /// empty-path sentinel honoured by [`Self::profile_library_dir`] so
    /// those harnesses don't trip on a `save_to` write to nowhere.
    fn persist_last_profile(&mut self) -> Result<()> {
        if self.settings_path.as_os_str().is_empty() {
            return Ok(());
        }
        self.settings.last_profile = self.state.read().profile_path.clone();
        self.settings.save_to(&self.settings_path)?;
        Ok(())
    }

    fn profile_path_for_name(&self, name: &str) -> PathBuf {
        self.profile_library_dir()
            .join(format!("{}.toml", sanitize_filename(name)))
    }

    pub(super) fn profile_origin_for_path(&self, path: &Path) -> ProfileOrigin {
        if path.starts_with(self.profile_library_dir()) {
            ProfileOrigin::Library
        } else {
            ProfileOrigin::External
        }
    }

    pub(super) fn refresh_profile_library_rows(&self) -> Result<()> {
        let library_dir = self.profile_library_dir();
        let active_path = self.state.read().profile_path.clone();
        let rows = list_profiles_in(&library_dir)?
            .into_iter()
            .map(|profile| {
                let is_active = active_path
                    .as_ref()
                    .is_some_and(|path| path == &profile.path);
                let (mode_count, last_edited_at) = read_profile_metadata(&profile.path);
                ProfileLibraryRow {
                    name: profile.name,
                    path: profile.path,
                    origin: ProfileOrigin::Library,
                    is_active,
                    mode_count,
                    last_edited_at,
                }
            })
            .collect();
        self.state.write().profile_library_rows = rows;
        Ok(())
    }

    /// Sweep every snapshot namespace's pending-delete subdir for
    /// expired entries.
    ///
    /// Library namespaces come from listing `<library_dir>/*.toml` and
    /// resolving each via `snapshots_dir_for`. External namespaces come
    /// from listing `<config_dir>/external_snapshots/`. A failure on a
    /// single namespace is logged and skipped so one corrupt namespace
    /// cannot block startup or profile load.
    fn purge_all_namespaces(&self) {
        let max_age = chrono::Duration::days(PENDING_DELETE_RETENTION_DAYS);
        let library_dir = self.profile_library_dir();
        if let Ok(entries) = std::fs::read_dir(&library_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_toml = path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"));
                if !is_toml {
                    continue;
                }
                match crate::snapshot::fs::snapshots_dir_for(&path) {
                    Ok(snap_dir) => {
                        let pending_dir = snap_dir.join(PENDING_SUBDIR);
                        if let Err(e) = purge_expired_pending_deletes(&pending_dir, max_age) {
                            tracing::warn!(
                                target: "snapshot",
                                profile_path = %path.display(),
                                error = %e,
                                "snapshot.purge.library_failure"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "snapshot",
                            profile_path = %path.display(),
                            error = %e,
                            "snapshot.purge.library_resolve_failure"
                        );
                    }
                }
            }
        }
        let external_root = crate::settings::AppSettings::config_dir().join("external_snapshots");
        if let Ok(entries) = std::fs::read_dir(&external_root) {
            for entry in entries.flatten() {
                let snap_dir = entry.path();
                if !snap_dir.is_dir() {
                    continue;
                }
                let pending_dir = snap_dir.join(PENDING_SUBDIR);
                if let Err(e) = purge_expired_pending_deletes(&pending_dir, max_age) {
                    tracing::warn!(
                        target: "snapshot",
                        snap_dir = %snap_dir.display(),
                        error = %e,
                        "snapshot.purge.external_failure"
                    );
                }
            }
        }
    }

    /// Resolve the active profile's snapshot target tuple
    /// `(profile_path, namespace_dir)` for snapshot lifecycle commands.
    ///
    /// Returns `None` when no profile is loaded; the caller should log
    /// and skip the command in that case rather than treating the
    /// missing target as an error.
    fn resolved_snapshot_target(&self) -> Option<(PathBuf, PathBuf)> {
        let state = self.state.read();
        let path = state.profile_path.as_ref()?.clone();
        let namespace_dir = match resolve_snapshot_namespace(&state) {
            Ok(dir) => dir,
            Err(e) => {
                tracing::warn!(
                    target: "snapshot",
                    profile_path = %path.display(),
                    error = %e,
                    "snapshot.namespace.resolution_failed"
                );
                return None;
            }
        };
        Some((path, namespace_dir))
    }

    pub(super) fn refresh_active_snapshot_rows(&self) -> Result<()> {
        let namespace_dir = {
            let state = self.state.read();
            if state.profile_path.is_none() {
                None
            } else {
                Some(resolve_snapshot_namespace(&state)?)
            }
        };
        let rows = if let Some(namespace_dir) = namespace_dir {
            list_visible(&namespace_dir)?
                .into_iter()
                .map(|snapshot| ActiveSnapshotRow {
                    id: snapshot.id,
                    kind: snapshot.kind,
                    label: snapshot.label,
                    taken_at: snapshot.taken_at,
                    pinned: snapshot.pinned,
                })
                .collect()
        } else {
            Vec::new()
        };
        self.state.write().active_snapshot_rows = rows;
        Ok(())
    }

    /// Update a mapping in the active profile and persist to disk.
    fn set_mapping(
        &self,
        input: &InputAddress,
        mode: &str,
        name: Option<String>,
        actions: Vec<Action>,
    ) {
        let mut state = self.state.write();

        if state.active_profile.is_none() {
            tracing::warn!("cannot set mapping: no profile loaded");
            return;
        }

        let Some(path) = state.profile_path.clone() else {
            tracing::warn!("cannot save mapping: no profile path");
            return;
        };

        let profile = state.active_profile.as_mut().expect("checked above");
        profile.set_mapping(input, mode, name, actions);

        if let Err(e) = profile.save(&path) {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to save mapping to profile"
            );
        }
    }

    fn reorder_mapping_in_group(
        &self,
        input: &InputAddress,
        mode: &str,
        target_index_in_group: usize,
    ) {
        let mut state = self.state.write();

        if state.active_profile.is_none() {
            tracing::warn!("cannot reorder mapping: no profile loaded");
            return;
        }

        let Some(path) = state.profile_path.clone() else {
            tracing::warn!("cannot save reorder: no profile path");
            return;
        };

        let profile = state.active_profile.as_mut().expect("checked above");
        let moved = profile.reorder_mapping_in_group(input, mode, target_index_in_group);

        // Skip the file write on no-ops (same-position, single-element
        // group, unknown mapping). Mirrors the SetMapping fast path:
        // persistence cost is paid only when the profile actually changed.
        if !moved {
            return;
        }

        if let Err(e) = profile.save(&path) {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to save reordered mapping to profile"
            );
        }
    }

    /// Remove a mapping from the active profile and persist to disk if
    /// the underlying `Profile::remove_mapping` reported a change.
    fn remove_mapping(&self, input: &InputAddress, mode: &str) {
        let mut state = self.state.write();

        if state.active_profile.is_none() {
            tracing::warn!(target: "f8::mapping_list", "cannot remove mapping: no profile loaded");
            return;
        }

        let Some(path) = state.profile_path.clone() else {
            tracing::warn!(target: "f8::mapping_list", "cannot remove mapping: no profile path");
            return;
        };

        let profile = state.active_profile.as_mut().expect("checked above");
        if !profile.remove_mapping(input, mode) {
            // No-op fast path: nothing to persist.
            return;
        }

        if let Err(e) = profile.save(&path) {
            tracing::warn!(
                target: "f8::mapping_list",
                path = %path.display(),
                error = %e,
                "failed to save profile after RemoveMapping",
            );
        }
    }

    /// Apply a bulk-map command. See `EngineCommand::SetMappingsBulk`
    /// for the four-step contract.
    ///
    /// Returns `()`, matching `set_mapping`'s shape. Snapshot and save
    /// errors surface to the user via the warnings channel rather than
    /// `?`, because the parent command-drain loop swallows arm errors
    /// and the user's recovery path is a manual Restore via the
    /// snapshot index UI.
    fn set_mappings_bulk(&self, entries: &[crate::action::BulkMapEntry], snapshot_label: String) {
        // Step 0: clone the profile path and namespace dir. The read guard
        // drops at the end of this block. Do not hold any state lock
        // during `crate::snapshot::create_in` and `crate::snapshot::prune_in`,
        // which perform disk I/O that must run lock-free (mirrors
        // `engine/run.rs` RestoreSnapshot at lines 687-700).
        let Some((path, namespace_dir)) = self.resolved_snapshot_target() else {
            tracing::warn!(target: "bulk_map", "SetMappingsBulk: no profile loaded, ignoring");
            self.state
                .write()
                .warnings
                .push("Bulk-map ignored: no profile loaded".to_owned());
            return;
        };

        // Step 1: pre-save in-memory profile so the on-disk body
        // matches the user's pre-bulk authored state. Without this,
        // the snapshot in step 2 captures whatever happened to be on
        // disk last (which may be older than the in-memory state if
        // any caller deferred a save).
        {
            let state = self.state.read();
            if let Some(profile) = state.active_profile.as_ref() {
                if let Err(e) = profile.save(&path) {
                    tracing::warn!(
                        target: "bulk_map",
                        path = %path.display(),
                        error = ?e,
                        "SetMappingsBulk: pre-snapshot save failed; aborting"
                    );
                    drop(state);
                    self.state.write().warnings.push(
                        "Bulk-map aborted: could not save profile before snapshot".to_owned(),
                    );
                    return;
                }
            } else {
                return;
            }
        }

        // Step 2: take the recovery snapshot. Abort if it fails so the
        // user never ends up with bulk-applied mappings and no
        // snapshot to roll back to.
        match crate::snapshot::create_in(
            &path,
            &namespace_dir,
            crate::snapshot::SnapshotKind::AutoBeforeBulkMap,
            Some(snapshot_label),
            &self.settings.snapshot,
        ) {
            Ok(_) => {
                let _ = crate::snapshot::prune_in(&namespace_dir, &self.settings.snapshot);
            }
            Err(e) => {
                tracing::warn!(
                    target: "bulk_map",
                    error = ?e,
                    "SetMappingsBulk: AutoBeforeBulkMap snapshot failed; aborting apply"
                );
                self.state
                    .write()
                    .warnings
                    .push("Bulk-map aborted: could not create recovery snapshot".to_owned());
                return;
            }
        }

        // Step 3: apply upserts and persist (second save).
        let mut state = self.state.write();
        let Some(profile) = state.active_profile.as_mut() else {
            return;
        };
        profile.set_mappings_bulk(entries);
        if let Err(e) = profile.save(&path) {
            tracing::warn!(
                target: "bulk_map",
                path = %path.display(),
                error = ?e,
                "SetMappingsBulk: post-bulk save failed; in-memory state holds bulk; recovery via Restore"
            );
            state.warnings.push(
                "Bulk-map applied in memory but disk save failed; reload to revert".to_owned(),
            );
        }
    }

    /// Persist the current calibration store into the loaded profile and save to disk.
    fn save_calibrations_to_profile(&self) {
        let mut state = self.state.write();

        if state.active_profile.is_none() {
            tracing::warn!("cannot save calibrations: no profile loaded");
            return;
        }

        let Some(path) = state.profile_path.clone() else {
            tracing::warn!("cannot save calibrations: no profile path");
            return;
        };

        // Rebuild CalibrationEntry list from the runtime store.
        let mut entries = Vec::new();
        for device_id in state.calibrations.device_ids() {
            for (axis, cal) in state.calibrations.get_for_device(device_id) {
                entries.push(CalibrationEntry::from_calibration(
                    device_id.clone(),
                    axis,
                    cal,
                ));
            }
        }

        let profile = state.active_profile.as_mut().expect("checked above");
        profile.set_calibrations(entries);

        if let Err(e) = profile.save(&path) {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to save calibrations to profile"
            );
        }
    }

    /// Read the current engine status from shared state.
    fn read_status(&self) -> EngineStatus {
        self.state.read().engine_status
    }

    /// Update device list in shared state from hotplug events.
    fn handle_hotplug(&mut self, events: &[HotplugEvent]) {
        for event in events {
            match event {
                HotplugEvent::Connected { info, diagnostics } => {
                    // Skip vJoy virtual HID devices, InputForge controls
                    // them through the output system, not as input devices.
                    if info.name.to_ascii_lowercase().contains("vjoy") {
                        continue;
                    }

                    let record = self.upsert_device_record(info, diagnostics);
                    let mut state = self.state.write();
                    state.device_registry.insert(info.id.clone(), record);
                    // Update existing or add new.
                    if let Some(dev) = state.devices.iter_mut().find(|d| d.info.id == info.id) {
                        dev.info = info.clone();
                        dev.connected = true;
                        dev.diagnostics = diagnostics.clone();
                    } else {
                        state.devices.push(DeviceState {
                            info: info.clone(),
                            connected: true,
                            diagnostics: diagnostics.clone(),
                        });
                    }
                }
                HotplugEvent::Disconnected(id) => {
                    let mut state = self.state.write();
                    if let Some(dev) = state.devices.iter_mut().find(|d| d.info.id == *id) {
                        dev.connected = false;
                    }
                    state.input_cache.evict_device(id);
                }
            }
        }
    }

    fn upsert_device_record(
        &mut self,
        info: &DeviceInfo,
        diagnostics: &DeviceDiagnostics,
    ) -> crate::settings::DeviceRecord {
        let record = crate::settings::DeviceRecord {
            info: info.clone(),
            diagnostics: diagnostics.clone(),
            last_seen_unix_ms: Some(current_unix_ms()),
        };
        self.settings
            .device_registry
            .insert(info.id.clone(), record.clone());
        if let Err(error) = self.settings.save_to(&self.settings_path) {
            tracing::warn!(
                target: "engine",
                error = %error,
                device = %info.id.0,
                "failed to persist remembered device registry"
            );
        }
        record
    }
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().try_into().unwrap_or(u64::MAX)
        })
}

fn pipeline_input_for_mapping(
    mapping: &Mapping,
    event: &InputEvent,
    state: &AppState,
) -> Option<(f64, InputValue)> {
    if mapping.input == event.source {
        let current_value = resolve_input_value(event, &state.calibrations);
        return Some((current_value, event.value.clone()));
    }

    let cached_inputs = state.input_cache.clone_compact();
    let input_value = cached_input_value(&cached_inputs, &mapping.input)?;
    let current_value = match &input_value {
        InputValue::Axis { .. } => {
            let cached_event = InputEvent {
                source: mapping.input.clone(),
                value: input_value.clone(),
                timestamp: event.timestamp,
            };
            resolve_input_value(&cached_event, &state.calibrations)
        }
        InputValue::Button { pressed } => {
            if *pressed {
                1.0
            } else {
                0.0
            }
        }
        InputValue::Hat { .. } => 0.0,
    };

    Some((current_value, input_value))
}

fn cached_input_value(entries: &[InputCacheEntry], address: &InputAddress) -> Option<InputValue> {
    entries
        .iter()
        .find(|entry| entry.address == *address)
        .map(|entry| entry.value.clone())
}

/// Resolve the pipeline input value from an event, applying calibration if available.
fn resolve_input_value(event: &InputEvent, calibrations: &DeviceCalibrationStore) -> f64 {
    match &event.value {
        InputValue::Axis { value, .. } => {
            let raw = value.value();
            // Invariant: events come from real device sources (Backend::poll
            // emits `Bound` addresses from device-tracked sources); `Unbound`
            // only originates from palette-seeded mapping primaries that
            // never produce events.
            let InputAddress::Bound { device, input } = &event.source else {
                unreachable!(
                    "invariant: input event source always Bound (Backend::poll emits Bound from device-tracked sources)"
                );
            };
            if let InputId::Axis { index } = input {
                calibrations
                    .get(device, *index)
                    .map_or(raw, |cal| cal.apply(raw))
            } else {
                raw
            }
        }
        InputValue::Button { pressed } => {
            if *pressed {
                1.0
            } else {
                0.0
            }
        }
        InputValue::Hat { .. } => 0.0,
    }
}
