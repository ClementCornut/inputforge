//! Render an `InputAddress` to a human-readable "Device . Input" label.
//!
//! Used by the F8 mapping-list row (second line, muted).

use std::borrow::Cow;

use inputforge_core::types::{InputAddress, InputId};

use crate::context::ConfigSnapshot;

/// Standard HID usage-page ordering. Axes 0-7 map to the names below;
/// higher indices fall back to `Ax {index}`. Ported from the legacy
/// `inputforge-gui::panels::device_view::HID_AXIS_LABELS` so axis-name
/// presentation stays consistent across the rewrite.
const HID_AXIS_LABELS: [&str; 8] = ["X", "Y", "Z", "Rot X", "Rot Y", "Rot Z", "Sldr", "Dial"];

fn axis_label(index: u8) -> Cow<'static, str> {
    let i = usize::from(index);
    if i < HID_AXIS_LABELS.len() {
        Cow::Borrowed(HID_AXIS_LABELS[i])
    } else {
        Cow::Owned(format!("Ax {i}"))
    }
}

/// Format an `InputAddress` against the current snapshot's device list.
///
/// - Connected device: `"<device.name> · <input-label>"`.
/// - Missing device: `"<DeviceId> · <input-label>"`. Caller's CSS may
///   italicize via `.if-row__source--unknown` to flag the gap.
pub(crate) fn format(addr: &InputAddress, cfg: &ConfigSnapshot) -> String {
    let (device_label, input_label) = split_label(addr, cfg);
    format!("{device_label} \u{00b7} {input_label}")
}

/// Split form of `format`: returns `(device_label, input_label)` so callers
/// can render the two cells separately. The captured-input chip in the F8
/// `AddInline` pad uses this — the input identifier needs its own layout
/// cell so it stays visible when the device name truncates.
pub(crate) fn split_label(addr: &InputAddress, cfg: &ConfigSnapshot) -> (String, String) {
    let device_label = match cfg.devices.iter().find(|d| d.info.id == addr.device) {
        Some(device) => device.info.name.clone(),
        None => addr.device.0.clone(),
    };
    let input_label = match addr.input {
        InputId::Axis { index } => axis_label(index).into_owned(),
        InputId::Button { index } => format!("Btn {}", index + 1),
        InputId::Hat { index } => format!("Hat {index}"),
    };
    (device_label, input_label)
}

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{AxisPolarity, DeviceId, DeviceInfo};

    fn cfg_with_device(name: &str, did: &str) -> ConfigSnapshot {
        ConfigSnapshot {
            devices: vec![DeviceState {
                info: DeviceInfo {
                    id: DeviceId(did.to_owned()),
                    name: name.to_owned(),
                    axes: 8,
                    buttons: 32,
                    hats: 1,
                    instance_path: None,
                    axis_polarities: vec![AxisPolarity::Bipolar; 8],
                },
                connected: true,
            }],
            ..ConfigSnapshot::default()
        }
    }

    #[test]
    fn format_axis_uses_hid_label() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Axis { index: 2 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle \u{00b7} Z");
    }

    #[test]
    fn format_axis_above_hid_range_falls_back() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Axis { index: 12 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle \u{00b7} Ax 12");
    }

    #[test]
    fn format_button_one_indexed() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Button { index: 3 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle \u{00b7} Btn 4");
    }

    #[test]
    fn format_hat_zero_indexed() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Hat { index: 0 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle \u{00b7} Hat 0");
    }

    #[test]
    fn format_missing_device_falls_back_to_device_id() {
        let cfg = ConfigSnapshot::default();
        let addr = InputAddress {
            device: DeviceId("tfm-disconnected".to_owned()),
            input: InputId::Button { index: 0 },
        };
        assert_eq!(format(&addr, &cfg), "tfm-disconnected \u{00b7} Btn 1");
    }

    #[test]
    fn split_label_returns_device_and_input_separately() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let (device, input) = split_label(&addr, &cfg);
        assert_eq!(device, "TFM Throttle");
        assert_eq!(input, "X");
    }

    #[test]
    fn split_label_button_one_indexed() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Button { index: 3 },
        };
        let (device, input) = split_label(&addr, &cfg);
        assert_eq!(device, "TFM Throttle");
        assert_eq!(input, "Btn 4");
    }

    #[test]
    fn split_label_missing_device_falls_back_to_id() {
        let cfg = ConfigSnapshot::default();
        let addr = InputAddress {
            device: DeviceId("ghost-dev".to_owned()),
            input: InputId::Hat { index: 0 },
        };
        let (device, input) = split_label(&addr, &cfg);
        assert_eq!(device, "ghost-dev");
        assert_eq!(input, "Hat 0");
    }
}
