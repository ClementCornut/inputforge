//! The single ownership teardown boundary. Never short-circuit a release sequence.
use super::Engine;
use crate::{
    error::{EngineError, Result},
    output::OutputFailure,
    state::EngineStatus,
};
use std::fmt::Write;
impl Engine {
    pub(super) fn cleanup_session(&mut self, retain: bool) -> Result<()> {
        self.state.write().engine_status = EngineStatus::Stopped;
        let mut errors = Vec::new();
        let result = self.input.release();
        self.collect_output_error(&mut errors, result);
        self.state.write().session.captured.clear();
        self.callbacks.clear();
        self.gesture_dispatcher.clear_all();
        self.event_buffer.clear();
        self.output_buffer.clear();
        self.pending_output_refresh = false;
        self.mode_state.clear_temporary();
        for action in self.output_state.release_all() {
            let result = super::output_handler::dispatch_output_action(
                action,
                &mut self.output_state,
                self.keyboard.as_mut(),
                self.mouse.as_mut(),
            );
            self.collect_output_error(&mut errors, result);
        }
        let result = self.keyboard.stop();
        self.collect_output_error(&mut errors, result);
        let result = self.mouse.stop();
        self.collect_output_error(&mut errors, result);
        // Neutralize the complete controller batch, never flush pending non-neutral axes through a button setter.
        if retain && errors.is_empty() {
            let result = self.output.neutralize();
            self.collect_output_error(&mut errors, result);
        }
        if !retain || !errors.is_empty() {
            let result = self.output.stop();
            self.collect_output_error(&mut errors, result);
        }
        if errors.is_empty() {
            self.output_state = super::output_state::OutputRuntimeState::default();
        }
        let mut state = self.state.write();
        self.mode_state
            .current()
            .clone_into(&mut state.current_mode);
        state.output_cache.clear();
        state.output_activity.clear();
        if !retain || !errors.is_empty() {
            state.session.output_active = false;
            state.session.output_layout.clear();
        }
        if errors.is_empty() {
            Ok(())
        } else {
            state.engine_status = EngineStatus::Faulted;
            Err(EngineError::OutputFailed {
                reason: errors.join("; "),
            })
        }
    }

    pub(super) fn release_all_held_outputs(&mut self) -> Result<()> {
        let mut errors = Vec::new();
        for action in self.output_state.release_all() {
            let result = super::output_handler::dispatch_output_action(
                action,
                &mut self.output_state,
                self.keyboard.as_mut(),
                self.mouse.as_mut(),
            );
            self.collect_output_error(&mut errors, result);
        }
        for (owner, address) in self.output_state.set_button_owners() {
            let result = self.release_virtual_owner(owner, address);
            self.collect_output_error(&mut errors, result);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(EngineError::OutputFailed {
                reason: errors.join("; "),
            })
        }
    }

    pub(super) fn release_virtual_owner(
        &mut self,
        owner: crate::pipeline::OutputOwner,
        address: crate::types::OutputAddress,
    ) -> Result<()> {
        use crate::types::{HatDirection, OutputId};
        match address.output {
            OutputId::Button { id } => self.output.set_button(address.device, id, false)?,
            OutputId::Hat { id } => {
                self.output
                    .set_hat(address.device, id, HatDirection::Center)?;
            }
            OutputId::Axis { .. } => return Ok(()),
        }
        let mut state = self.state.write();
        state.output_activity.clear_owner(&owner);
        match address.output {
            OutputId::Button { id } => state.output_cache.set_button(address.device, id, false),
            OutputId::Hat { id } => {
                state
                    .output_cache
                    .set_hat(address.device, id, HatDirection::Center);
            }
            OutputId::Axis { .. } => {}
        }
        self.output_state.commit_set_button(owner, address, false);
        Ok(())
    }

    pub(super) fn fault(&mut self, primary: &EngineError) {
        if let EngineError::InjectionFailed { failure } = primary {
            self.remember_output_failure(failure);
        }
        let cleanup = self.cleanup_session(false);
        let mut message = primary.to_string();
        if let Err(error) = cleanup {
            let _ = write!(message, "; cleanup: {error}");
        }
        let mut state = self.state.write();
        state.engine_status = EngineStatus::Faulted;
        state.session.error = Some(message.clone());
        state.warnings.push(message);
    }

    pub(super) fn remember_output_failure(&mut self, failure: &OutputFailure) {
        let mut state = self.state.write();
        if let Some(existing) = state
            .session
            .output_failures
            .iter_mut()
            .find(|existing| existing.output == failure.output)
        {
            existing.cleanup.push(failure.to_string());
            existing.cleanup.extend(failure.cleanup.iter().cloned());
        } else {
            state.session.output_failures.push(failure.clone());
        }
    }

    fn collect_output_error(&mut self, errors: &mut Vec<String>, result: Result<()>) {
        if let Err(error) = result {
            if let EngineError::InjectionFailed { failure } = &error {
                self.remember_output_failure(failure);
            }
            errors.push(error.to_string());
        }
    }
}
