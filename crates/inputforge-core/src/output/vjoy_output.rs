// Rust guideline compliant 2026-03-03

use std::collections::{HashMap, HashSet};
use std::fmt;

use vjoy::{ButtonState, Device, HatState, VJoy};

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
///
/// State changes from [`OutputSink::set_axis`], [`OutputSink::set_button`],
/// and [`OutputSink::set_hat`] are cached in memory. Call
/// [`OutputSink::flush`] to write all dirty device states to the driver
/// in a single IPC call per device.
pub struct VJoyOutput {
    vjoy: VJoy,
    active_devices: HashSet<u8>,
    /// Cached device states, modified in-place by set methods.
    cached_states: HashMap<u8, Device>,
    /// Devices whose cached state has been modified since the last flush.
    dirty_devices: HashSet<u8>,
}

impl fmt::Debug for VJoyOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VJoyOutput")
            .field("active_devices", &self.active_devices)
            .field("dirty_devices", &self.dirty_devices)
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
            cached_states: HashMap::new(),
            dirty_devices: HashSet::new(),
        })
    }
}

impl VJoyOutput {
    /// Ensure the vJoy device is in the cache, creating it on first use.
    fn ensure_device(&mut self, device_id: u8) -> Result<()> {
        if self.cached_states.contains_key(&device_id) {
            return Ok(());
        }
        let id = u32::from(device_id);
        let state = self.vjoy.get_device_state(id).map_err(|e| {
            tracing::debug!("vJoy device error: {e:?}");
            EngineError::VJoyDeviceUnavailable { device_id }
        })?;
        self.cached_states.insert(device_id, state);
        self.active_devices.insert(device_id);
        Ok(())
    }
}

impl OutputSink for VJoyOutput {
    fn create_device(&mut self, config: &VirtualDeviceConfig) -> Result<()> {
        let id = u32::from(config.device_id);
        // The `vjoy` crate acquires all available devices during
        // `VJoy::from_default_dll_location` (see `VJoy::fetch_devices`).
        // Explicit acquire is not needed; the private API does not expose it.
        let state = self.vjoy.get_device_state(id).map_err(|e| {
            tracing::debug!("vJoy device error: {e:?}");
            EngineError::VJoyDeviceUnavailable {
                device_id: config.device_id,
            }
        })?;

        // Log the requested capabilities. vJoy device configuration is
        // managed externally (via vJoyConf), so we cannot enforce these
        // values here.
        tracing::debug!(
            device_id = config.device_id,
            axes = ?config.axes,
            button_count = config.button_count,
            hat_count = config.hat_count,
            "vJoy device config received; actual capabilities are set externally via vJoyConf"
        );

        self.cached_states.insert(config.device_id, state);
        self.active_devices.insert(config.device_id);
        Ok(())
    }

    fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()> {
        let axis_id = vjoy_axis_id(axis);
        let vjoy_value = axis_value_to_vjoy(value);

        self.ensure_device(device)?;
        let state = self.cached_states.get_mut(&device).expect("just ensured");
        state
            .set_axis(axis_id, vjoy_value)
            .map_err(|e| EngineError::OutputFailed {
                reason: format!("vJoy device {device} axis {axis:?} (id {axis_id}): {e:?}"),
            })?;
        self.dirty_devices.insert(device);
        Ok(())
    }

    fn set_button(&mut self, device: u8, button: u8, pressed: bool) -> Result<()> {
        let button_state = if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };

        self.ensure_device(device)?;
        let state = self.cached_states.get_mut(&device).expect("just ensured");
        state
            .set_button(button, button_state)
            .map_err(|e| EngineError::OutputFailed {
                reason: format!("vJoy device {device} button {button}: {e:?}"),
            })?;
        self.dirty_devices.insert(device);
        Ok(())
    }

    fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) -> Result<()> {
        let hat_state = hat_direction_to_vjoy(direction);

        self.ensure_device(device)?;
        let state = self.cached_states.get_mut(&device).expect("just ensured");
        state
            .set_hat(hat, hat_state)
            .map_err(|e| EngineError::OutputFailed {
                reason: format!("vJoy device {device} hat {hat}: {e:?}"),
            })?;
        self.dirty_devices.insert(device);
        Ok(())
    }

    fn release_device(&mut self, device: u8) -> Result<()> {
        // Flush any unflushed dirty state before releasing the device.
        if self.dirty_devices.contains(&device)
            && let Some(state) = self.cached_states.get(&device)
        {
            self.vjoy
                .update_device_state(state)
                .map_err(|e| EngineError::OutputFailed {
                    reason: format!("vJoy device {device} flush on release: {e:?}"),
                })?;
        }

        // The `vjoy` crate relinquishes all devices in its `Drop` impl.
        // Per-device relinquish is not exposed by the `vjoy` crate API.
        self.active_devices.remove(&device);
        self.cached_states.remove(&device);
        self.dirty_devices.remove(&device);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let mut first_err = None;
        let mut flushed = Vec::new();

        for &device_id in &self.dirty_devices {
            if let Some(state) = self.cached_states.get(&device_id) {
                match self.vjoy.update_device_state(state) {
                    Ok(()) => {
                        flushed.push(device_id);
                    }
                    Err(e) => {
                        if first_err.is_none() {
                            first_err = Some(EngineError::OutputFailed {
                                reason: format!("vJoy device {device_id}: {e:?}"),
                            });
                        }
                    }
                }
            } else {
                debug_assert!(false, "dirty device {device_id} has no cached state");
                tracing::warn!(
                    device_id,
                    "dirty device has no cached state, skipping flush"
                );
            }
        }

        for id in flushed {
            self.dirty_devices.remove(&id);
        }

        first_err.map_or(Ok(()), Err)
    }

    fn list_devices(&self) -> Vec<VirtualDeviceConfig> {
        // `build_device_configs` needs &mut self because the vjoy crate's
        // `hid_usage()` accessor takes &mut. Work around by cloning devices.
        self.vjoy
            .devices()
            .map(|device| {
                let axes: Vec<VJoyAxis> = device
                    .axes()
                    .filter_map(|axis| {
                        // Axis fields are pub(crate) in the vjoy crate, but the
                        // Display impl prints the HID usage. Use the known fixed
                        // order: axes are stored in HID usage order (0x30..0x37)
                        // and only present axes are included. Map by position
                        // against the full axis table.
                        //
                        // We parse the Display output "Axis ID: N | ..." to get
                        // the 1-based axis ID.
                        let display = format!("{axis}");
                        let id_str = display
                            .strip_prefix("Axis ID: ")
                            .and_then(|s| s.split(" |").next());
                        id_str.and_then(|s| s.parse::<u32>().ok()).and_then(|id| {
                            // axis IDs are 1-based, AXES_HID_USAGE is 0-indexed
                            let hid = [0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37];
                            hid.get((id - 1) as usize)
                                .copied()
                                .and_then(hid_usage_to_vjoy_axis)
                        })
                    })
                    .collect();
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "vJoy supports max 128 buttons and 4 hats, both fit in u8"
                )]
                VirtualDeviceConfig {
                    device_id: device.id() as u8,
                    axes,
                    button_count: device.num_buttons() as u8,
                    hat_count: device.num_hats() as u8,
                }
            })
            .collect()
    }
}

/// Flush all dirty devices on drop, logging any errors that occur.
impl Drop for VJoyOutput {
    fn drop(&mut self) {
        for &device_id in &self.dirty_devices {
            if let Some(state) = self.cached_states.get(&device_id)
                && let Err(e) = self.vjoy.update_device_state(state)
            {
                tracing::warn!(device_id, error = ?e, "failed to flush vJoy device on drop");
            }
        }
    }
}

/// Map a vJoy HID usage code to a [`VJoyAxis`] variant.
///
/// Returns `None` for unrecognised usage codes.
fn hid_usage_to_vjoy_axis(hid: u32) -> Option<VJoyAxis> {
    match hid {
        0x30 => Some(VJoyAxis::X),
        0x31 => Some(VJoyAxis::Y),
        0x32 => Some(VJoyAxis::Z),
        0x33 => Some(VJoyAxis::Rx),
        0x34 => Some(VJoyAxis::Ry),
        0x35 => Some(VJoyAxis::Rz),
        0x36 => Some(VJoyAxis::Slider0),
        0x37 => Some(VJoyAxis::Slider1),
        _ => None,
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
/// Non-finite inputs (NaN, infinity) are treated as zero.
fn axis_value_to_vjoy(value: f64) -> i32 {
    let safe = if value.is_finite() { value } else { 0.0 };
    let clamped = safe.clamp(-1.0, 1.0);
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
    fn axis_nan_maps_to_center() {
        let center = axis_value_to_vjoy(0.0);
        assert_eq!(axis_value_to_vjoy(f64::NAN), center);
    }

    #[test]
    fn axis_infinity_maps_to_center() {
        let center = axis_value_to_vjoy(0.0);
        assert_eq!(axis_value_to_vjoy(f64::INFINITY), center);
        assert_eq!(axis_value_to_vjoy(f64::NEG_INFINITY), center);
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
