//! Synchronize adapter control descriptions with persisted logical slots.
use super::Engine;
use crate::{
    error::Result,
    profile::controllers::ControllerConfig,
    settings::{AxisSetting, DeviceRecord},
    types::DeviceInfo,
};
impl Engine {
    pub(super) fn sync_controller_bindings(&mut self) -> Result<()> {
        let mut bindings = Vec::new();
        for info in self.input.enumerate_devices() {
            if let Some(table) = self.input.binding_table(&info.id)? {
                bindings.push(table);
            }
        }
        self.state.write().session.bindings.clone_from(&bindings);
        let Some(mut profile) = self.state.read().active_profile.clone() else {
            return Ok(());
        };
        let old = profile.controllers().cloned();
        let mut config = old.clone().unwrap_or_else(|| ControllerConfig {
            virtual_devices: self.output.list_devices(),
            selected: profile
                .mappings()
                .iter()
                .flat_map(super::dependencies::mapping_dependencies)
                .filter_map(|a| a.device().cloned())
                .fold(Vec::new(), |mut ids, id| {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                    ids
                }),
            ..ControllerConfig::default()
        });
        if config.legacy_axis_settings {
            let previous = self.settings.device_registry.clone();
            for binding in &config.bindings {
                let record = self
                    .settings
                    .device_registry
                    .entry(binding.device.clone())
                    .or_insert_with(|| DeviceRecord {
                        axis_settings: vec![],
                        info: DeviceInfo {
                            id: binding.device.clone(),
                            name: binding.device.0.clone(),
                            axes: u8::try_from(binding.axes.len()).unwrap_or(u8::MAX),
                            buttons: u8::try_from(binding.buttons.len()).unwrap_or(u8::MAX),
                            hats: u8::try_from(binding.hats.len()).unwrap_or(u8::MAX),
                            instance_path: None,
                            axis_polarities: vec![],
                        },
                        diagnostics: crate::types::DeviceDiagnostics::default(),
                        last_seen_unix_ms: None,
                    });
                for axis in &binding.axes {
                    if !record.axis_settings.iter().any(|s| s.code == axis.code) {
                        record.axis_settings.push(AxisSetting {
                            code: axis.code,
                            detected: None,
                            override_polarity: Some(axis.polarity),
                        });
                    }
                }
            }
            if let Err(error) = self.settings.save_to(&self.settings_path) {
                self.settings.device_registry = previous;
                return Err(error);
            }
            self.state
                .write()
                .device_registry
                .clone_from(&self.settings.device_registry);
            config.legacy_axis_settings = false;
        }
        for table in bindings {
            if let Some(saved) = config
                .bindings
                .iter_mut()
                .find(|b| b.device == table.device)
            {
                *saved = table;
            } else {
                config.bindings.push(table);
            }
        }
        // Keep legacy selected devices resolvable while disconnected; their frozen descriptions are retained.

        if old.as_ref() == Some(&config) {
            return Ok(());
        }
        profile.set_controllers(config.clone())?;
        let virtual_devices = self.published_output_layout(&config.virtual_devices);
        let mut state = self.state.write();
        if state.active_profile_origin == Some(crate::state::ProfileOrigin::Library)
            && let Some(path) = &state.profile_path
        {
            profile.save(path)?;
        }
        state.virtual_devices = virtual_devices;
        state.active_profile = Some(profile);
        state.session.generation = state.session.generation.wrapping_add(1);
        Ok(())
    }
}
