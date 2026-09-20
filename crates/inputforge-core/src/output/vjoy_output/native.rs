//! Conversion between shared controls and native vJoy configuration.
use crate::{
    processing::lerp_range,
    types::{HatDirection, VJoyAxis, VirtualDeviceConfig},
};
use vjoy::{HatState, VJoy};
// vJoy's documented axis range is 0x0001 through 0x8000.
const VJOY_AXIS_MIN: f64 = 1.0;
const VJOY_AXIS_MAX: f64 = 32_768.0;
pub(super) fn configs(vjoy: &VJoy) -> Vec<VirtualDeviceConfig> {
    vjoy.devices_cloned()
        .into_iter()
        .filter_map(|mut device| {
            Some(VirtualDeviceConfig {
                device_id: u8::try_from(device.id()).ok()?,
                axes: device
                    .axes_mut()
                    .filter_map(|axis| hid_usage_to_vjoy_axis(axis.hid_usage()))
                    .collect(),
                button_count: u8::try_from(device.num_buttons()).ok()?,
                hat_count: u8::try_from(device.num_hats()).ok()?,
            })
        })
        .collect()
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
pub(super) fn vjoy_axis_id(axis: VJoyAxis) -> u32 {
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
pub(super) fn axis_value_to_vjoy(value: f64) -> i32 {
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
pub(super) fn hat_direction_to_vjoy(direction: HatDirection) -> HatState {
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
    pub(super) fn vjoy_axis_ids_are_one_based_and_distinct() {
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
