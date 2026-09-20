use super::Error;
use crate::types::{VJoyAxis, VirtualDeviceConfig};

/// Return an eight-axis, 32-button, four-hat configuration for a logical slot.
///
/// This performs no I/O. [`super::Output::new`] validates the supplied slot.
#[must_use]
pub fn default_config(slot: u8) -> VirtualDeviceConfig {
    use VJoyAxis::{Rx, Ry, Rz, Slider0, Slider1, X, Y, Z};
    VirtualDeviceConfig {
        device_id: slot,
        axes: vec![X, Y, Z, Rx, Ry, Rz, Slider0, Slider1],
        button_count: 32,
        hat_count: 4,
    }
}

pub(super) fn validate(
    mut configs: Vec<VirtualDeviceConfig>,
) -> Result<Vec<VirtualDeviceConfig>, Error> {
    if configs.is_empty() || configs.len() > 16 {
        return Err(Error::invalid(
            "configure",
            None,
            "expected 1 through 16 virtual devices",
        ));
    }
    configs.sort_by_key(|c| c.device_id);
    for (index, config) in configs.iter().enumerate() {
        let slot = Some(config.device_id);
        if !(1..=16).contains(&config.device_id)
            || index > 0 && configs[index - 1].device_id == config.device_id
        {
            return Err(Error::invalid(
                "configure",
                slot,
                "slots must be unique and within 1..=16",
            ));
        }
        if !(2..=53).contains(&config.button_count) || config.hat_count > 4 {
            return Err(Error::invalid(
                "configure",
                slot,
                "expected 2..=53 buttons and 0..=4 hats",
            ));
        }
        if config
            .axes
            .iter()
            .enumerate()
            .any(|(i, axis)| config.axes[..i].contains(axis))
        {
            return Err(Error::invalid("configure", slot, "duplicate axis"));
        }
    }
    Ok(configs)
}

pub(super) const fn axis_code(axis: VJoyAxis) -> u16 {
    match axis {
        VJoyAxis::X => 0,
        VJoyAxis::Y => 1,
        VJoyAxis::Z => 2,
        VJoyAxis::Rx => 3,
        VJoyAxis::Ry => 4,
        VJoyAxis::Rz => 5,
        VJoyAxis::Slider0 => 6,
        VJoyAxis::Slider1 => 7,
    }
}

pub(super) fn button_code(button: u8) -> Option<u16> {
    // Linux input-event-codes.h: skip reserved gaps and all gamepad/digitizer/KEY codes.
    match button {
        1..=12 => Some(0x120 + u16::from(button) - 1),
        13 => Some(0x12f),
        14..=53 => Some(0x2c0 + u16::from(button) - 14),
        _ => None,
    }
}

pub(super) fn axes(config: &VirtualDeviceConfig) -> Vec<(u16, i32, i32)> {
    config
        .axes
        .iter()
        .map(|&axis| (axis_code(axis), -32767, 32767))
        .chain((0..config.hat_count).flat_map(|hat| {
            let x = 0x10 + u16::from(hat) * 2;
            [(x, -1, 1), (x + 1, -1, 1)]
        }))
        .collect()
}

pub(super) fn name(slot: u8) -> String {
    format!("InputForge Virtual Controller {slot}")
}
pub(super) fn phys(slot: u8) -> String {
    format!("inputforge/controller/v1/slot/{slot}")
}
