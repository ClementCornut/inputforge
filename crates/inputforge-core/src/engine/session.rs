//! Shared routing lifecycle; input monitoring is independent of output ownership.
use super::{Engine, EngineCommand};
use crate::{error::Result, profile::controllers::invalid, state::EngineStatus};

impl Engine {
    pub(super) fn handle_session_command(&mut self, cmd: &EngineCommand) -> Result<bool> {
        if self.shutdown {
            return Ok(true);
        }
        match cmd {
            EngineCommand::Shutdown => {
                self.shutdown = true;
                self.cleanup_session(false)?;
            }
            EngineCommand::Activate | EngineCommand::Retry => {
                self.begin_session()?;
            }
            EngineCommand::Deactivate => {
                self.cleanup_session(true)?;
            }
            EngineCommand::RefreshInput => {
                self.input.refresh()?;
                self.state.write().session.error = None;
            }
            EngineCommand::ApplyControllerChanges => {
                self.apply_controller_changes()?;
            }
            EngineCommand::SetControllerConfig(config) => {
                self.save_controller_config(config.clone())?;
            }
            EngineCommand::SelectControllers(ids) => {
                self.require_stopped()?;
                let mut config = self
                    .state
                    .read()
                    .active_profile
                    .as_ref()
                    .and_then(|p| p.controllers())
                    .cloned()
                    .unwrap_or_default();
                for id in ids {
                    if !config.bindings.iter().any(|b| &b.device == id) {
                        config.bindings.push(
                            self.input
                                .binding_table(id)?
                                .ok_or_else(|| invalid("Controller capabilities unavailable"))?,
                        );
                    }
                }
                config.selected.clone_from(ids);
                self.save_controller_config(config)?;
            }
            EngineCommand::ConfirmInputBinding { device, input } => {
                self.confirm_input_binding(device, input)?;
            }
            EngineCommand::SetAxisPolarity {
                device,
                axis,
                polarity,
            } => self.set_axis_polarity(device, *axis, *polarity)?,
            EngineCommand::DetectAxis { device, axis } => self.detect_axis(device, *axis)?,
            _ => {
                self.before_profile_edit(cmd)?;
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn begin_session(&mut self) -> Result<()> {
        if self.read_status() == EngineStatus::Running {
            return Ok(());
        }
        if self.read_status() == EngineStatus::Faulted {
            self.cleanup_session(false)?;
        }
        self.sync_controller_bindings()?;
        let config = self
            .state
            .read()
            .active_profile
            .as_ref()
            .ok_or_else(|| invalid("Create or load a profile first"))?
            .controllers()
            .cloned()
            .unwrap_or_default();
        config.validate()?;
        self.validate_output_config(&config.virtual_devices)?;
        if self
            .state
            .read()
            .session
            .requires_output_reconfiguration(&config.virtual_devices)
        {
            return Err(invalid(
                "Apply controller changes in Devices before starting routing",
            ));
        }
        let result = (|| {
            if !self.state.read().session.output_active && !config.virtual_devices.is_empty() {
                self.start_outputs(&config.virtual_devices)?;
            }
            let selected: Vec<_> = config
                .selected
                .iter()
                .filter(|id| self.input.is_device_connected(id))
                .cloned()
                .collect();
            if self.input.supports_exclusive() && !selected.is_empty() {
                self.input.acquire(&selected)?;
                self.state.write().session.captured = selected;
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.fault(&error);
            return Err(error);
        }
        self.block_sampled_inputs();
        self.pending_output_refresh = true;
        let mut state = self.state.write();
        state.engine_status = EngineStatus::Running;
        state.session.error = None;
        state.session.generation = state.session.generation.wrapping_add(1);
        Ok(())
    }
}
