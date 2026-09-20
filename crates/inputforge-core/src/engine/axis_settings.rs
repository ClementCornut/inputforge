//! Controller-wide detection and overrides; adapters only supply trustworthy samples.
use super::Engine;
use crate::{
    error::Result,
    profile::controllers::invalid,
    settings::AxisSetting,
    types::{AxisPolarity, DeviceId, InputEvent, InputId, InputValue},
};
impl Engine {
    fn axis_code(&self, device: &DeviceId, axis: u8) -> Result<u16> {
        self.input
            .binding_table(device)?
            .and_then(|b| b.axes.get(usize::from(axis)).map(|a| a.code))
            .or_else(|| {
                self.state
                    .read()
                    .active_profile
                    .as_ref()?
                    .controllers()?
                    .bindings
                    .iter()
                    .find(|b| &b.device == device)?
                    .axes
                    .get(usize::from(axis))
                    .map(|a| a.code)
            })
            .ok_or_else(|| invalid("Axis is unavailable"))
    }
    pub(super) fn set_axis_polarity(
        &mut self,
        device: &DeviceId,
        axis: u8,
        polarity: Option<AxisPolarity>,
    ) -> Result<()> {
        self.change_axis_setting(device, axis, |setting| setting.override_polarity = polarity)
    }
    pub(super) fn detect_axis(&mut self, device: &DeviceId, axis: u8) -> Result<()> {
        self.input.request_axis_sample(device, axis)?;
        self.change_axis_setting(device, axis, |setting| setting.detected = None)
    }
    pub(super) fn observe_axis(&mut self, device: &DeviceId, axis: u8, value: f64) -> Result<()> {
        let code = self.axis_code(device, axis)?;
        if self.settings.device_registry.get(device).is_some_and(|r| {
            r.axis_settings
                .iter()
                .any(|a| a.code == code && a.detected.is_some())
        }) {
            return Ok(());
        }
        self.change_axis_setting(device, axis, |setting| setting.detect(value))
    }
    fn change_axis_setting(
        &mut self,
        device: &DeviceId,
        axis: u8,
        update: impl FnOnce(&mut AxisSetting),
    ) -> Result<()> {
        let code = self.axis_code(device, axis)?;
        let before = self.settings.clone();
        let record = self
            .settings
            .device_registry
            .get_mut(device)
            .ok_or_else(|| invalid("Controller is unavailable"))?;
        let index = record
            .axis_settings
            .iter()
            .position(|a| a.code == code)
            .unwrap_or_else(|| {
                record.axis_settings.push(AxisSetting {
                    code,
                    detected: None,
                    override_polarity: None,
                });
                record.axis_settings.len() - 1
            });
        update(&mut record.axis_settings[index]);
        if let Err(error) = self.settings.save_to(&self.settings_path) {
            self.settings = before;
            self.state
                .write()
                .warnings
                .push(format!("Could not save axis settings: {error}"));
            return Err(error);
        }
        self.state
            .write()
            .device_registry
            .clone_from(&self.settings.device_registry);
        let entries = self.state.read().input_cache.clone_compact();
        for entry in entries {
            if entry.address.device() == Some(device) {
                let mut event = InputEvent {
                    source: entry.address,
                    value: entry.value,
                    timestamp: (self.now)(),
                };
                self.apply_axis_polarity(&mut event);
                self.state
                    .write()
                    .input_cache
                    .update(&event.source, &event.value);
            }
        }
        self.pending_output_refresh = true;
        Ok(())
    }
    pub(super) fn apply_axis_polarity(&self, event: &mut InputEvent) {
        let (Some(device), Some(InputId::Axis { index })) =
            (event.source.device(), event.source.input_id())
        else {
            return;
        };
        let Ok(code) = self.axis_code(device, *index) else {
            return;
        };
        let Some(setting) = self
            .settings
            .device_registry
            .get(device)
            .and_then(|r| r.axis_settings.iter().find(|a| a.code == code))
        else {
            return;
        };
        if let InputValue::Axis { polarity, .. } = &mut event.value {
            *polarity = setting.effective();
        }
        if let Some(info) = self
            .state
            .write()
            .devices
            .iter_mut()
            .find(|d| &d.info.id == device)
            && let Some(polarity) = info.info.axis_polarities.get_mut(usize::from(*index))
        {
            *polarity = setting.effective();
        }
    }
}
