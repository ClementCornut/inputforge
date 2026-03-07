// Rust guideline compliant 2026-03-06

//! Engine main loop and per-frame tick logic.
//!
//! The [`Engine::run`] method drives the main loop, calling
//! [`Engine::tick`] each iteration. Separating the single-frame
//! logic into `tick` makes unit testing straightforward without
//! dealing with the loop or sleep.

use std::sync::mpsc;
use std::time::Duration;

use crate::callbacks::ReleaseCallback;
use crate::device::traits::HotplugEvent;
use crate::error::Result;
use crate::mode::resolve_mapping;
use crate::pipeline::{self, PipelineContext};
use crate::profile::{CalibrationEntry, Profile};
use crate::state::{DeviceCalibrationStore, DeviceState, EngineStatus};
use crate::types::{InputEvent, InputId, InputValue};

use super::Engine;
use super::command::EngineCommand;
use super::output_handler::{
    process_pipeline_outputs, record_outputs_to_cache, refresh_axes_for_mode_change,
};

/// Target poll interval for the engine loop.
///
/// 1 ms provides responsive input handling at ~1000 Hz without
/// excessive CPU usage. The OS scheduler may add jitter.
const POLL_INTERVAL: Duration = Duration::from_millis(1);

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
        let (mappings, mode_tree) = {
            let state = self.state.read();
            match &state.active_profile {
                Some(profile) => (profile.mappings().to_vec(), profile.modes().clone()),
                None => return Ok(()),
            }
        };

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

            // Resolve mapping for this input in the current mode.
            let Some(mapping) = resolve_mapping(
                &mappings,
                &event.source,
                self.mode_state.current(),
                &mode_tree,
            ) else {
                continue;
            };

            // Single lock acquisition for calibration lookup and pipeline context.
            let guard = self.state.read();
            let current_value = resolve_input_value(event, &guard.calibrations);

            let mut ctx = PipelineContext {
                current_value,
                input_value: event.value.clone(),
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
            )?;

            self.output_buffer.extend_from_slice(&outputs);

            // If mode changed (via pipeline output or release callbacks),
            // refresh all cached axes through the new mode.
            if result.mode_changed || callbacks_changed_mode {
                let mut guard = self.state.write();
                let state: &mut crate::state::AppState = &mut guard;
                refresh_axes_for_mode_change(
                    &state.input_cache,
                    &mappings,
                    self.mode_state.current(),
                    &mode_tree,
                    self.output.as_mut(),
                    &mut state.output_cache,
                )?;
                self.output_buffer.clear();
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
    fn handle_command(&mut self, cmd: EngineCommand) -> Result<()> {
        match cmd {
            EngineCommand::LoadProfile(path) => {
                let profile = Profile::load(&path)?;
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
                state.profile_path = Some(path);
                state.current_mode = startup_mode;
                state.input_cache.clear();
                state.output_cache.clear();
            }
            EngineCommand::Activate | EngineCommand::Resume => {
                let mut state = self.state.write();
                state.engine_status = EngineStatus::Running;
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
            EngineCommand::Shutdown => {
                self.shutdown = true;
            }
        }
        Ok(())
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
                    // Skip vJoy virtual HID devices — InputForge controls
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

/// Resolve the pipeline input value from an event, applying calibration if available.
fn resolve_input_value(event: &InputEvent, calibrations: &DeviceCalibrationStore) -> f64 {
    match &event.value {
        InputValue::Axis { value } => {
            let raw = value.value();
            if let InputId::Axis { index } = &event.source.input {
                calibrations
                    .get(&event.source.device, *index)
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
