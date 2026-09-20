//! Explicit changes to the lifetime of virtual controller handles.
use super::Engine;
use crate::{error::Result, profile::controllers::invalid, types::VirtualDeviceConfig};

impl Engine {
    pub(super) fn published_output_layout(
        &self,
        configured: &[VirtualDeviceConfig],
    ) -> Vec<VirtualDeviceConfig> {
        if self.output.capabilities().configurable {
            configured.to_vec()
        } else {
            self.output.list_devices()
        }
    }

    pub(super) fn validate_output_config(&self, configs: &[VirtualDeviceConfig]) -> Result<()> {
        let caps = self.output.capabilities();
        for slot in configs {
            if slot.button_count < caps.min_buttons
                || slot.button_count > caps.max_buttons
                || slot.hat_count > caps.max_hats
            {
                return Err(invalid("Virtual controller exceeds backend capabilities"));
            }
        }
        Ok(())
    }

    pub(super) fn start_outputs(&mut self, configs: &[VirtualDeviceConfig]) -> Result<()> {
        if configs.is_empty() {
            return Ok(());
        }
        self.output.start(configs)?;
        self.output.neutralize()?;
        let published = self.published_output_layout(configs);
        let active = if self.output.capabilities().configurable {
            published.clone()
        } else {
            published
                .iter()
                .filter(|native| {
                    configs
                        .iter()
                        .any(|requested| requested.device_id == native.device_id)
                })
                .cloned()
                .collect()
        };
        let mut state = self.state.write();
        state.session.output_active = true;
        state.session.output_layout = active;
        state.virtual_devices = published;
        Ok(())
    }

    pub(super) fn apply_controller_changes(&mut self) -> Result<()> {
        self.require_stopped()?;
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
        if self.state.read().session.output_active
            && !self
                .state
                .read()
                .session
                .requires_output_reconfiguration(&config.virtual_devices)
        {
            return Ok(());
        }
        self.cleanup_session(false)?;
        if let Err(error) = self.start_outputs(&config.virtual_devices) {
            self.fault(&error);
            return Err(error);
        }
        let mut state = self.state.write();
        state.session.error = None;
        state.session.generation = state.session.generation.wrapping_add(1);
        Ok(())
    }
}
