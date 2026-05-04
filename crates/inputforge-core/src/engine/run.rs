// Rust guideline compliant 2026-03-06

//! Engine main loop and per-frame tick logic.
//!
//! The [`Engine::run`] method drives the main loop, calling
//! [`Engine::tick`] each iteration. Separating the single-frame
//! logic into `tick` makes unit testing straightforward without
//! dealing with the loop or sleep.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use crate::action::{Action, Mapping};
use crate::callbacks::ReleaseCallback;
use crate::device::traits::HotplugEvent;
use crate::error::Result;
use crate::pipeline::{self, PipelineContext};
use crate::profile::{CalibrationEntry, Profile};
use crate::state::{AppState, DeviceCalibrationStore, DeviceState, EngineStatus, InputCacheEntry};
use crate::types::{InputAddress, InputEvent, InputId, InputValue};

use super::Engine;
use super::command::EngineCommand;
use super::dependencies::active_mappings_for_event;
use super::output_handler::{
    process_pipeline_outputs, record_outputs_to_cache, refresh_axes_for_mode_change,
};

/// Target poll interval for the engine loop.
///
/// 1 ms provides responsive input handling at ~1000 Hz without
/// excessive CPU usage. The OS scheduler may add jitter.
const POLL_INTERVAL: Duration = Duration::from_millis(1);

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
    /// Processes pending commands, polls input, routes events through
    /// the mode tree and pipeline, and writes output. Call this
    /// directly in tests instead of [`run`](Self::run) to avoid the
    /// loop and sleep.
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
        // Clone mappings + mode tree to avoid holding the lock during processing.
        let (mappings, mode_tree, mode_forced) = {
            let state = self.state.read();
            match &state.active_profile {
                Some(profile) => (
                    profile.mappings().to_vec(),
                    profile.modes().clone(),
                    state.mode_force.is_some(),
                ),
                None => return Ok(()),
            }
        };

        // On first tick after activation, refresh all cached axis outputs so
        // vJoy reflects current physical device positions without waiting for
        // a new input event.
        self.apply_activation_refresh(&mappings, &mode_tree)?;

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
                            if !mode_forced {
                                self.mode_state.pop_temporary();
                            }
                        }
                        ReleaseCallback::Custom(f) => f(),
                    }
                }
                callbacks_changed_mode = self.mode_state.current() != mode_before_callbacks;
            }

            let active_mappings = active_mappings_for_event(
                &mappings,
                &event.source,
                self.mode_state.current(),
                &mode_tree,
            );
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
                pipeline::execute_pipeline(&mapping.actions, &mut ctx);
                let outputs = std::mem::take(&mut ctx.outputs);
                drop(guard);

                // Process pipeline outputs.
                let result = process_pipeline_outputs(
                    &outputs,
                    self.output.as_mut(),
                    self.keyboard.as_mut(),
                    &mut self.mode_state,
                    &mode_tree,
                    &mut self.callbacks,
                    &event.source,
                    mode_forced,
                )?;

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
                        &mode_tree,
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
    fn apply_activation_refresh(
        &mut self,
        mappings: &[Mapping],
        mode_tree: &crate::mode::ModeTree,
    ) -> Result<()> {
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
            mode_tree,
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
                self.reload_profile_from_disk(&path)?;
                // A forced-mode override should not survive a profile change.
                self.state.write().mode_force = None;
                let _ = crate::snapshot::create(
                    &path,
                    crate::snapshot::SnapshotKind::AutoSessionStart,
                    None,
                    &self.settings.snapshot,
                )?;
                let _ = crate::snapshot::prune(&path, &self.settings.snapshot)?;
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
            EngineCommand::ForceMode { mode } => {
                // D15: idempotent same-mode; rotate on different-mode.
                let already_same = self
                    .state
                    .read()
                    .mode_force
                    .as_ref()
                    .is_some_and(|f| f.mode == mode);
                if already_same {
                    return Ok(());
                }
                // Read mode tree from active_profile (may be absent, return early).
                let tree = if let Some(p) = self.state.read().active_profile.as_ref() {
                    p.modes().clone()
                } else {
                    tracing::warn!(
                        target: "engine",
                        "ForceMode dispatched with no profile; ignoring"
                    );
                    return Ok(());
                };
                self.mode_state.switch_to(&mode, &tree)?;
                let mut state = self.state.write();
                state.mode_force = Some(crate::state::ForcedMode { mode: mode.clone() });
                mode.clone_into(&mut state.current_mode);
                drop(state);
                self.pending_output_refresh = true;
                tracing::info!(target: "engine", mode = %mode, "ForceMode applied");
            }
            EngineCommand::ReleaseMode => {
                self.state.write().mode_force = None;
                tracing::info!(target: "engine", "ReleaseMode applied");
            }
            EngineCommand::ReloadSettings => {
                self.settings = crate::settings::AppSettings::load_from(&self.settings_path);
                tracing::info!(target: "engine", "settings reloaded");
            }
            EngineCommand::CreateSnapshot { kind, label } => {
                let path = self.state.read().profile_path.clone();
                if let Some(path) = path {
                    let _ = crate::snapshot::create(&path, kind, label, &self.settings.snapshot)?;
                    let _ = crate::snapshot::prune(&path, &self.settings.snapshot)?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "CreateSnapshot dispatched with no profile loaded"
                    );
                }
            }
            EngineCommand::DeleteSnapshot { id } => {
                let path = self.state.read().profile_path.clone();
                if let Some(path) = path {
                    crate::snapshot::delete(&path, &id)?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "DeleteSnapshot dispatched with no profile loaded"
                    );
                }
            }
            EngineCommand::PinSnapshot { id, pinned } => {
                let path = self.state.read().profile_path.clone();
                if let Some(path) = path {
                    crate::snapshot::pin(&path, &id, pinned)?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "PinSnapshot dispatched with no profile loaded"
                    );
                }
            }
            EngineCommand::RenameSnapshot { id, label } => {
                let path = self.state.read().profile_path.clone();
                if let Some(path) = path {
                    crate::snapshot::rename(&path, &id, label)?;
                } else {
                    tracing::warn!(
                        target: "snapshot",
                        "RenameSnapshot dispatched with no profile loaded"
                    );
                }
            }
            EngineCommand::AddMode { name, parent } => {
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
                let parent_name = parent
                    .clone()
                    .unwrap_or_else(|| profile.modes().root().name().to_owned());
                let new_tree = profile.modes().with_added_child(&parent_name, &name)?;
                profile.set_modes(new_tree);
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
                tracing::info!(target: "engine", mode = %name, parent = %parent_name, "AddMode applied");
            }
            EngineCommand::RenameMode { from, to } => {
                // Validate both names against the same policy. Without
                // this, an oversized `from` would fall through to
                // `with_renamed` and surface as `ModeNotFound`, leaking
                // an internal detail (the name doesn't match a tree node)
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
                //   1. `with_renamed` clones the tree and validates the
                //      collision (returns Err without mutating). Must run
                //      first so a name collision doesn't leave a partial
                //      mapping rewrite behind.
                //   2. `rename_mode_refs` pre-validates cycles across all
                //      mappings (using its internal `check_cycle_rename`)
                //      then mutates mappings + startup_mode in one pass.
                //      Atomic on Err via the pre-validation pass.
                //   3. `set_modes` swaps in the new tree last. Single-shot,
                //      infallible, once the cascade has succeeded the
                //      tree replacement cannot fail partway.
                // Reordering risks: (1)→(3)→(2) commits a tree against
                // stale mapping references if cycle-validation later
                // rejects; (2)→(1)→(3) mutates mappings before the tree
                // is validated against the new name, which would orphan
                // mappings on collision.
                // Step 1: tree rewrite (errors on missing-from / collision).
                let new_tree = profile.modes().with_renamed(&from, &to)?;
                // Step 2: pre-validate + cascade across mappings + startup.
                let touched = profile.rename_mode_refs(&from, &to)?;
                // Step 3: swap the new tree in last.
                profile.set_modes(new_tree);

                // Step 3: runtime-state cascade.
                if state.current_mode == from {
                    to.clone_into(&mut state.current_mode);
                }
                if let Some(force) = state.mode_force.as_mut() {
                    if force.mode == from {
                        to.clone_into(&mut force.mode);
                    }
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

                // Pre-validation.
                if profile.modes().root().name() == name {
                    return Err(crate::error::EngineError::InvalidConfig {
                        reason: "cannot delete root mode".to_owned(),
                    });
                }
                if !profile.modes().contains(&name) {
                    return Err(crate::error::EngineError::ModeNotFound { name: name.clone() });
                }

                // Compute the deleted set (subtree + name).
                let descendants = profile.modes().descendants_of(&name)?;
                let mut deleted: Vec<String> = descendants;
                deleted.push(name.clone());

                let startup = profile.settings().startup_mode().to_owned();
                if deleted.iter().any(|m| m == &startup) {
                    return Err(crate::error::EngineError::InvalidConfig {
                        reason: format!(
                            "cannot delete mode '{name}', its subtree contains startup mode '{startup}'"
                        ),
                    });
                }

                // Apply the tree mutation.
                let new_tree = profile.modes().with_subtree_removed(&name)?;
                profile.set_modes(new_tree);

                // Cascade-drop every mapping scoped to a deleted mode.
                let mut mappings_dropped = 0usize;
                for m in &deleted {
                    mappings_dropped += profile.remove_mappings_for_mode(m);
                }

                // Runtime state cascade.
                if deleted.iter().any(|m| m == &state.current_mode) {
                    startup.clone_into(&mut state.current_mode);
                }
                if state
                    .mode_force
                    .as_ref()
                    .is_some_and(|f| deleted.iter().any(|m| m == &f.mode))
                {
                    state.mode_force = None;
                }
                drop(state);

                // ModeState reset.
                if deleted.iter().any(|m| m == self.mode_state.current()) {
                    let tree = self
                        .state
                        .read()
                        .active_profile
                        .as_ref()
                        .map(|p| p.modes().clone());
                    if let Some(tree) = tree {
                        self.mode_state.switch_to(&startup, &tree)?;
                    }
                }
                self.mode_state.clear_stack_entries(&deleted);

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
                    modes_deleted = ?deleted,
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
                let path = self.state.read().profile_path.clone();
                let Some(path) = path else {
                    tracing::warn!(target: "snapshot", "RestoreSnapshot dispatched with no profile loaded");
                    return Ok(());
                };

                // Step 1, capture AutoBeforeRestore (always fires; never deduped).
                let auto = crate::snapshot::create(
                    &path,
                    crate::snapshot::SnapshotKind::AutoBeforeRestore,
                    None,
                    &self.settings.snapshot,
                )?;
                let _ = crate::snapshot::prune(&path, &self.settings.snapshot)?;

                // Step 2, strip meta + atomically write target body to live path.
                crate::snapshot::restore(&path, &id)?;

                // Step 3, reload from disk; auto-rollback on failure.
                if let Err(reload_err) = self.reload_profile_from_disk(&path) {
                    tracing::error!(
                        target: "snapshot",
                        ?reload_err,
                        "restore reload failed; rolling back to AutoBeforeRestore"
                    );
                    if let Some(auto_snap) = auto {
                        crate::snapshot::restore(&path, &auto_snap.id)?;
                        self.reload_profile_from_disk(&path)?;
                    }
                    return Err(reload_err);
                }

                // Successful restore clears mode_force (snapshot's mode tree may differ).
                self.state.write().mode_force = None;

                tracing::info!(
                    target: "snapshot",
                    id = %id,
                    "RestoreSnapshot complete"
                );
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
        }
        Ok(())
    }

    /// Reload the active profile from disk and rebuild dependent in-memory state.
    ///
    /// Resets calibrations, mode state, callbacks, and the active profile to
    /// match `path` on disk. Shared between `LoadProfile` and `RestoreSnapshot`.
    ///
    /// **Does not** touch `state.mode_force`, the caller is responsible for
    /// that policy decision.
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
        // Step 0: clone the profile path. The read guard drops at the
        // end of this `let`. Do not hold any state lock during
        // `crate::snapshot::create` and `crate::snapshot::prune`,
        // which perform disk I/O that must run lock-free (mirrors
        // `engine/run.rs` RestoreSnapshot at lines 687-700).
        let Some(path) = self.state.read().profile_path.clone() else {
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
        match crate::snapshot::create(
            &path,
            crate::snapshot::SnapshotKind::AutoBeforeBulkMap,
            Some(snapshot_label),
            &self.settings.snapshot,
        ) {
            Ok(_) => {
                let _ = crate::snapshot::prune(&path, &self.settings.snapshot);
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
    fn handle_hotplug(&self, events: &[HotplugEvent]) {
        let mut state = self.state.write();
        for event in events {
            match event {
                HotplugEvent::Connected(info) => {
                    // Skip vJoy virtual HID devices, InputForge controls
                    // them through the output system, not as input devices.
                    if info.name.to_ascii_lowercase().contains("vjoy") {
                        continue;
                    }

                    // Update existing or add new.
                    if let Some(dev) = state.devices.iter_mut().find(|d| d.info.id == info.id) {
                        dev.info = info.clone();
                        dev.connected = true;
                    } else {
                        state.devices.push(DeviceState {
                            info: info.clone(),
                            connected: true,
                        });
                    }
                }
                HotplugEvent::Disconnected(id) => {
                    if let Some(dev) = state.devices.iter_mut().find(|d| d.info.id == *id) {
                        dev.connected = false;
                    }
                    state.input_cache.evict_device(id);
                }
            }
        }
    }
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
