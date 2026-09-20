//! Configuration updates do not stop passive input monitoring.
use super::{Engine, EngineCommand};
use crate::{
    error::Result,
    profile::controllers::{ControllerConfig, invalid},
    state::EngineStatus,
};
impl Engine {
    pub(super) fn require_stopped(&self) -> Result<()> {
        let state = self.state.read();
        if matches!(
            state.engine_status,
            EngineStatus::Running | EngineStatus::Starting
        ) || !state.session.captured.is_empty()
        {
            return Err(invalid(
                "Stop routing before changing controllers or virtual capabilities",
            ));
        }
        Ok(())
    }

    pub(super) fn save_controller_config(&mut self, config: ControllerConfig) -> Result<()> {
        self.require_stopped()?;
        let (mut profile, path) = {
            let state = self.state.read();
            (
                state
                    .active_profile
                    .clone()
                    .ok_or_else(|| invalid("Create or load a profile first"))?,
                state.profile_path.clone(),
            )
        };
        let previous = profile
            .controllers()
            .map_or_else(Vec::new, |c| c.bindings.clone());
        let bindings = config.bindings.clone();
        let virtual_devices = self.published_output_layout(&config.virtual_devices);
        profile.set_controllers(config)?;
        self.input.configure(&bindings)?;
        if let Some(path) = path
            && let Err(error) = profile.save(&path)
        {
            self.input.configure(&previous)?;
            return Err(error);
        }
        let mut state = self.state.write();
        state.virtual_devices = virtual_devices;
        state.active_profile = Some(profile);
        state.session.error = None;
        state.session.generation = state.session.generation.wrapping_add(1);
        Ok(())
    }

    pub(super) fn confirm_input_binding(
        &mut self,
        device: &crate::types::DeviceId,
        input: &crate::types::InputId,
    ) -> Result<()> {
        let old = self
            .input
            .binding_table(device)?
            .ok_or_else(|| invalid("Controller unavailable"))?;
        if !old.unavailable.contains(input) {
            return Ok(());
        }
        let table = self.input.confirm_binding(device, input)?;
        let result = (|| {
            let mut state = self.state.write();
            if let Some(mut profile) = state.active_profile.clone() {
                let mut config = profile.controllers().cloned().unwrap_or_default();
                if let Some(saved) = config.bindings.iter_mut().find(|b| &b.device == device) {
                    *saved = table.clone();
                }
                profile.set_controllers(config)?;
                if let Some(path) = &state.profile_path {
                    profile.save(path)?;
                }
                state.active_profile = Some(profile);
            }
            if let Some(saved) = state
                .session
                .bindings
                .iter_mut()
                .find(|b| &b.device == device)
            {
                *saved = table;
            }
            state.session.generation = state.session.generation.wrapping_add(1);
            Ok(())
        })();
        if result.is_err() {
            let mut bindings = self.state.read().session.bindings.clone();
            if let Some(saved) = bindings.iter_mut().find(|b| &b.device == device) {
                *saved = old;
            }
            self.input.configure(&bindings)?;
        }
        self.pending_output_refresh = true;
        result
    }

    pub(super) fn before_profile_edit(&mut self, cmd: &EngineCommand) -> Result<()> {
        use EngineCommand as C;
        let replace = matches!(
            cmd,
            C::LoadProfile(_)
                | C::CreateProfile { .. }
                | C::LoadExternalProfileOnce(_)
                | C::AddExternalProfileToLibrary { .. }
                | C::RestoreSnapshot { .. }
        );
        let active_named = match cmd {
            C::DeleteProfile { name } => self
                .state
                .read()
                .active_profile
                .as_ref()
                .is_some_and(|p| p.name() == name),
            _ => false,
        };
        if replace || active_named {
            self.cleanup_session(true)?;
        }
        if matches!(cmd, C::SetCalibration { .. } | C::SaveCalibrations) {
            self.pending_output_refresh = self.read_status() == EngineStatus::Running;
        }
        Ok(())
    }
}
