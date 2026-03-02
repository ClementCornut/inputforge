// Rust guideline compliant 2026-03-03

use std::collections::HashSet;
use std::fmt;

use vjoy::{ButtonState, HatState, VJoy};

use crate::error::{EngineError, Result};
use crate::processing::lerp_range;
use crate::types::{HatDirection, VJoyAxis, VirtualDeviceConfig};

use super::traits::OutputSink;

/// Minimum vJoy axis value (0x0001).
///
/// The vJoy driver uses an unsigned 16-bit range where 0x0001 is the
/// minimum and 0x8000 is the maximum.
const VJOY_AXIS_MIN: f64 = 1.0;

/// Maximum vJoy axis value (0x8000).
const VJOY_AXIS_MAX: f64 = 32_768.0;

/// Output sink that writes to virtual vJoy devices.
pub struct VJoyOutput {
    vjoy: VJoy,
    active_devices: HashSet<u8>,
}

impl fmt::Debug for VJoyOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VJoyOutput")
            .field("active_devices", &self.active_devices)
            .finish_non_exhaustive()
    }
}

impl VJoyOutput {
    /// Create a new `VJoyOutput` by loading the vJoy driver.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::VJoyDriverMissing`] if the vJoy driver is not
    /// installed or the DLL cannot be loaded.
    pub fn new() -> Result<Self> {
        let vjoy = VJoy::from_default_dll_location().map_err(|e| {
            tracing::debug!("vJoy driver load failed: {e:?}");
            EngineError::VJoyDriverMissing
        })?;
        Ok(Self {
            vjoy,
            active_devices: HashSet::new(),
        })
    }
}

impl OutputSink for VJoyOutput {
    fn create_device(&mut self, config: &VirtualDeviceConfig) -> Result<()> {
        let id = u32::from(config.device_id);
        // Verify the device exists and is acquired.
        let _state = self.vjoy.get_device_state(id).map_err(|e| {
            tracing::debug!("vJoy device error: {e:?}");
            EngineError::VJoyDeviceUnavailable {
                device_id: config.device_id,
            }
        })?;
        self.active_devices.insert(config.device_id);
        Ok(())
    }

    fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()> {
        let id = u32::from(device);
        let axis_id = vjoy_axis_id(axis);
        let vjoy_value = axis_value_to_vjoy(value);

        let mut state = self.vjoy.get_device_state(id).map_err(|e| {
            tracing::debug!("vJoy device error: {e:?}");
            EngineError::VJoyDeviceUnavailable { device_id: device }
        })?;
        state
            .set_axis(axis_id, vjoy_value)
            .map_err(|e| EngineError::InvalidConfig {
                reason: format!("vJoy axis error: {e:?}"),
            })?;
        self.vjoy
            .update_device_state(&state)
            .map_err(|e| EngineError::InvalidConfig {
                reason: format!("vJoy update error: {e:?}"),
            })?;
        Ok(())
    }

    fn set_button(&mut self, device: u8, button: u8, pressed: bool) -> Result<()> {
        let id = u32::from(device);
        let button_state = if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };

        let mut state = self.vjoy.get_device_state(id).map_err(|e| {
            tracing::debug!("vJoy device error: {e:?}");
            EngineError::VJoyDeviceUnavailable { device_id: device }
        })?;
        state
            .set_button(button, button_state)
            .map_err(|e| EngineError::InvalidConfig {
                reason: format!("vJoy button error: {e:?}"),
            })?;
        self.vjoy
            .update_device_state(&state)
            .map_err(|e| EngineError::InvalidConfig {
                reason: format!("vJoy update error: {e:?}"),
            })?;
        Ok(())
    }

    fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) -> Result<()> {
        let id = u32::from(device);
        let hat_state = hat_direction_to_vjoy(direction);

        let mut state = self.vjoy.get_device_state(id).map_err(|e| {
            tracing::debug!("vJoy device error: {e:?}");
            EngineError::VJoyDeviceUnavailable { device_id: device }
        })?;
        state
            .set_hat(hat, hat_state)
            .map_err(|e| EngineError::InvalidConfig {
                reason: format!("vJoy hat error: {e:?}"),
            })?;
        self.vjoy
            .update_device_state(&state)
            .map_err(|e| EngineError::InvalidConfig {
                reason: format!("vJoy update error: {e:?}"),
            })?;
        Ok(())
    }

    fn release_device(&mut self, device: u8) -> Result<()> {
        self.active_devices.remove(&device);
        Ok(())
    }
}

/// Map a [`VJoyAxis`] variant to the corresponding 1-based vJoy axis ID.
fn vjoy_axis_id(axis: VJoyAxis) -> u32 {
    match axis {
        VJoyAxis::X => 1,
        VJoyAxis::Y => 2,
        VJoyAxis::Z => 3,
        VJoyAxis::Rx => 4,
        VJoyAxis::Ry => 5,
        VJoyAxis::Rz => 6,
        VJoyAxis::Slider0 => 7,
        VJoyAxis::Slider1 => 8,
    }
}

/// Convert a normalized axis value ([-1.0, 1.0]) to the vJoy integer range.
///
/// Uses [`lerp_range`] to map from \[-1.0, 1.0\] to \[0x0001, 0x8000\].
fn axis_value_to_vjoy(value: f64) -> i32 {
    let clamped = value.clamp(-1.0, 1.0);
    let raw = lerp_range(clamped, -1.0, 1.0, VJOY_AXIS_MIN, VJOY_AXIS_MAX);
    // The result is in range [1.0, 32768.0], fitting safely in i32.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "result is within 1..=32768 after clamp and lerp"
    )]
    {
        raw.round() as i32
    }
}

/// Convert a [`HatDirection`] to a vJoy continuous hat state.
///
/// Uses hundredths of degrees (100 = 1 degree). `u32::MAX` represents the
/// centered/neutral position.
fn hat_direction_to_vjoy(direction: HatDirection) -> HatState {
    HatState::Continuous(match direction {
        HatDirection::Center => u32::MAX,
        HatDirection::N => 0,
        HatDirection::NE => 4_500,
        HatDirection::E => 9_000,
        HatDirection::SE => 13_500,
        HatDirection::S => 18_000,
        HatDirection::SW => 22_500,
        HatDirection::W => 27_000,
        HatDirection::NW => 31_500,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_neg_one_maps_to_vjoy_min() {
        assert_eq!(axis_value_to_vjoy(-1.0), 1);
    }

    #[test]
    fn axis_one_maps_to_vjoy_max() {
        assert_eq!(axis_value_to_vjoy(1.0), 32_768);
    }

    #[test]
    fn axis_zero_maps_to_vjoy_center() {
        let center = axis_value_to_vjoy(0.0);
        // Midpoint of [1, 32768] is 16384.5, rounds to 16385.
        assert!((16_384..=16_385).contains(&center));
    }

    #[test]
    fn axis_clamps_out_of_range() {
        assert_eq!(axis_value_to_vjoy(-2.0), axis_value_to_vjoy(-1.0));
        assert_eq!(axis_value_to_vjoy(5.0), axis_value_to_vjoy(1.0));
    }

    #[test]
    fn hat_center_maps_to_neutral() {
        assert_eq!(
            hat_direction_to_vjoy(HatDirection::Center),
            HatState::Continuous(u32::MAX)
        );
    }

    #[test]
    fn hat_north_maps_to_zero() {
        assert_eq!(
            hat_direction_to_vjoy(HatDirection::N),
            HatState::Continuous(0)
        );
    }

    #[test]
    fn hat_all_directions_are_distinct() {
        let dirs = [
            HatDirection::Center,
            HatDirection::N,
            HatDirection::NE,
            HatDirection::E,
            HatDirection::SE,
            HatDirection::S,
            HatDirection::SW,
            HatDirection::W,
            HatDirection::NW,
        ];
        let values: Vec<_> = dirs.iter().map(|d| hat_direction_to_vjoy(*d)).collect();
        for (i, a) in values.iter().enumerate() {
            for (j, b) in values.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "directions {i} and {j} should differ");
                }
            }
        }
    }

    #[test]
    fn vjoy_axis_ids_are_one_based_and_distinct() {
        let axes = [
            VJoyAxis::X,
            VJoyAxis::Y,
            VJoyAxis::Z,
            VJoyAxis::Rx,
            VJoyAxis::Ry,
            VJoyAxis::Rz,
            VJoyAxis::Slider0,
            VJoyAxis::Slider1,
        ];
        let ids: Vec<_> = axes.iter().map(|a| vjoy_axis_id(*a)).collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
