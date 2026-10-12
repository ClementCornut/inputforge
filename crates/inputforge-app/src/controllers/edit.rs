//! Controller mutations always start from the current profile.
use super::{MAX_SLOTS, policy};
use anyhow::Result;
use inputforge_core::{
    engine::EngineCommand,
    output::validation::validate_layout,
    state::AppState,
    types::{VJoyAxis, VirtualDeviceConfig},
};
use inputforge_gui_dx::controllers::Action;

// Keep the existing default virtual layout and vJoy-compatible slot range.
const AXES: [VJoyAxis; 8] = [
    VJoyAxis::X,
    VJoyAxis::Y,
    VJoyAxis::Z,
    VJoyAxis::Rx,
    VJoyAxis::Ry,
    VJoyAxis::Rz,
    VJoyAxis::Slider0,
    VJoyAxis::Slider1,
];

pub(super) fn resolve(state: &AppState, action: Action) -> Result<Option<EngineCommand>> {
    if !policy::editable(state) {
        return Ok(None);
    }
    let mut config = state
        .active_profile
        .as_ref()
        .and_then(|p| p.controllers())
        .cloned()
        .unwrap_or_default();
    let capabilities = state.session.controller_capabilities;
    match action {
        Action::Polarity { .. } | Action::Detect { .. } => return Ok(None),
        Action::Select { device, selected } => {
            let connected = state
                .devices
                .iter()
                .any(|d| d.connected && d.info.id == device);
            if !config.selected.contains(&device) && (!selected || !connected) {
                return Ok(None);
            }
            config.selected.retain(|id| id != &device);
            if selected {
                config.selected.push(device);
            }
            config.validate()?;
            return Ok(Some(EngineCommand::SelectControllers(config.selected)));
        }
        Action::Apply => {
            if !state
                .session
                .requires_output_reconfiguration(&config.virtual_devices)
            {
                return Ok(None);
            }
            config.validate()?;
            validate_layout(&config.virtual_devices, capabilities)?;
            return Ok(Some(EngineCommand::ApplyControllerChanges));
        }
        _ if !capabilities.configurable => return Ok(None),
        Action::Add => {
            let Some(slot) = (1..=MAX_SLOTS)
                .find(|slot| !config.virtual_devices.iter().any(|c| c.device_id == *slot))
            else {
                return Ok(None);
            };
            config.virtual_devices.push(VirtualDeviceConfig {
                device_id: slot,
                axes: AXES.to_vec(),
                button_count: 32_u8.clamp(capabilities.min_buttons, capabilities.max_buttons),
                hat_count: capabilities.max_hats,
            });
        }
        Action::Remove { slot } => {
            if !config.virtual_devices.iter().any(|c| c.device_id == slot) {
                return Ok(None);
            }
            config.virtual_devices.retain(|c| c.device_id != slot);
        }
        Action::Buttons { slot, value } => {
            let Some(target) = config
                .virtual_devices
                .iter_mut()
                .find(|c| c.device_id == slot)
            else {
                return Ok(None);
            };
            target.button_count = value.parse::<u8>()?;
        }
        Action::Hats { slot, value } => {
            let Some(target) = config
                .virtual_devices
                .iter_mut()
                .find(|c| c.device_id == slot)
            else {
                return Ok(None);
            };
            target.hat_count = value.parse::<u8>()?;
        }
        Action::Axis {
            slot,
            axis,
            enabled,
        } => {
            let Some(target) = config
                .virtual_devices
                .iter_mut()
                .find(|c| c.device_id == slot)
            else {
                return Ok(None);
            };
            target.axes.retain(|a| a != &axis);
            if enabled {
                target.axes.push(axis);
            }
        }
    }
    config.validate()?;
    validate_layout(&config.virtual_devices, capabilities)?;
    Ok(Some(EngineCommand::SetControllerConfig(config)))
}
