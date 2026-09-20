use super::{OutputSink, VJoyOutput};
use crate::{
    error::{EngineError, Result},
    types::HatDirection,
};
impl VJoyOutput {
    pub(super) fn neutralize_session(&mut self) -> Result<()> {
        if self.active_devices.is_empty() {
            return Ok(());
        }
        let mut errors = Vec::new();
        for config in self.list_devices() {
            if !self.active_devices.contains(&config.device_id) {
                continue;
            }
            for axis in config.axes {
                if let Err(e) = self.set_axis(config.device_id, axis, 0.0) {
                    errors.push(e.to_string());
                }
            }
            for button in 1..=config.button_count {
                if let Err(e) = self.set_button(config.device_id, button, false) {
                    errors.push(e.to_string());
                }
            }
            for hat in 1..=config.hat_count {
                if let Err(e) = self.set_hat(config.device_id, hat, HatDirection::Center) {
                    errors.push(e.to_string());
                }
            }
        }
        if let Err(e) = self.flush() {
            errors.push(e.to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(EngineError::OutputFailed {
                reason: errors.join("; "),
            })
        }
    }
}
