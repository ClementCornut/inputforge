//! Render an `InputAddress` to a human-readable "Device . Input" label.
//!
//! Used by the F8 mapping-list row (second line, muted).

use std::borrow::Cow;

use inputforge_core::types::{InputAddress, InputId};

use crate::context::ConfigSnapshot;

/// Standard HID usage-page ordering. Axes 0-7 map to the names below;
/// higher indices fall back to `Ax {index}`. Mirrors the conventional HID
/// usage-page names so axis presentation stays consistent across the GUI.
const HID_AXIS_LABELS: [&str; 8] = ["X", "Y", "Z", "Rot X", "Rot Y", "Rot Z", "Sldr", "Dial"];

/// Placeholder shown when the address has no binding selected yet
/// (`InputAddress::Unbound`). Stages added from the palette display this
/// instead of a misleading `Btn 1` sentinel until the user picks an input.
const UNBOUND_PLACEHOLDER: &str = "Unbound";

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
/// - `Unbound`: returns the literal `"Unbound"` placeholder (no separator).
pub(crate) fn format(addr: &InputAddress, cfg: &ConfigSnapshot) -> String {
    match addr {
        InputAddress::Unbound => UNBOUND_PLACEHOLDER.to_owned(),
        InputAddress::Bound { .. } => {
            let (device_label, input_label) = split_label(addr, cfg);
            format!("{device_label} \u{00b7} {input_label}")
        }
    }
}

/// Split form of `format`: returns `(device_label, input_label)` so callers
/// can render the two cells separately. The captured-input chip in the F8
/// `AddInline` pad uses this, the input identifier needs its own layout
/// cell so it stays visible when the device name truncates.
///
/// For `Unbound` returns `("", "Unbound")` so the device cell collapses and
/// the input cell carries the placeholder.
pub(crate) fn split_label(addr: &InputAddress, cfg: &ConfigSnapshot) -> (String, String) {
    let (device, input) = match addr {
        InputAddress::Bound { device, input } => (device, input),
        InputAddress::Unbound => return (String::new(), UNBOUND_PLACEHOLDER.to_owned()),
    };
    let device_label = cfg.device_display_name(device);
    let input_label = match input {
        InputId::Axis { index } => axis_label(*index).into_owned(),
        InputId::Button { index } => format!("Btn {}", index + 1),
        InputId::Hat { index } => format!("Hat {index}"),
    };
    (device_label, input_label)
}

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo};

    fn cfg_with_device(name: &str, did: &str) -> ConfigSnapshot {
        let device = DeviceState {
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
            diagnostics: DeviceDiagnostics::default(),
        };
        let device_display_names =
            std::collections::HashMap::from([(device.info.id.clone(), device.info.name.clone())]);
        ConfigSnapshot {
            devices: vec![device],
            device_display_names,
            ..ConfigSnapshot::default()
        }
    }

    #[test]
    fn format_axis_uses_hid_label() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress::Bound {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Axis { index: 2 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle \u{00b7} Z");
    }

    #[test]
    fn format_axis_above_hid_range_falls_back() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress::Bound {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Axis { index: 12 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle \u{00b7} Ax 12");
    }

    #[test]
    fn format_button_one_indexed() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress::Bound {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Button { index: 3 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle \u{00b7} Btn 4");
    }

    #[test]
    fn format_hat_zero_indexed() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress::Bound {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Hat { index: 0 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle \u{00b7} Hat 0");
    }

    #[test]
    fn format_missing_device_falls_back_to_device_id() {
        let cfg = ConfigSnapshot::default();
        let addr = InputAddress::Bound {
            device: DeviceId("tfm-disconnected".to_owned()),
            input: InputId::Button { index: 0 },
        };
        assert_eq!(format(&addr, &cfg), "tfm-disconnected \u{00b7} Btn 1");
    }

    #[test]
    fn split_label_returns_device_and_input_separately() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress::Bound {
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
        let addr = InputAddress::Bound {
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
        let addr = InputAddress::Bound {
            device: DeviceId("ghost-dev".to_owned()),
            input: InputId::Hat { index: 0 },
        };
        let (device, input) = split_label(&addr, &cfg);
        assert_eq!(device, "ghost-dev");
        assert_eq!(input, "Hat 0");
    }

    #[test]
    fn format_unbound_renders_placeholder() {
        // Locks in the original `Btn 1` bug fix: palette-seeded stages must
        // render the explicit `Unbound` placeholder rather than a misleading
        // sentinel button label.
        let cfg = ConfigSnapshot::default();
        assert_eq!(format(&InputAddress::Unbound, &cfg), "Unbound");
    }

    #[test]
    fn split_label_unbound_returns_empty_device_and_placeholder_input() {
        let cfg = ConfigSnapshot::default();
        let (device, input) = split_label(&InputAddress::Unbound, &cfg);
        assert_eq!(device, "");
        assert_eq!(input, "Unbound");
    }

    #[test]
    fn split_label_uses_alias_over_hardware_name() {
        // Regression guard for the device-alias display-name spec: this
        // call site must read `cfg.device_display_name(...)` (alias),
        // not `info.name` (hardware). Set both to clearly-distinct
        // strings so a silent revert to `info.name` would fail this
        // test.
        let device = DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Generic HID Joystick".to_owned(),
                axes: 4,
                buttons: 16,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 4],
            },
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        };
        let device_display_names = std::collections::HashMap::from([(
            device.info.id.clone(),
            "Throttle Quadrant".to_owned(),
        )]);
        let cfg = ConfigSnapshot {
            devices: vec![device],
            device_display_names,
            ..ConfigSnapshot::default()
        };
        let addr = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };

        let (device_label, input_label) = split_label(&addr, &cfg);
        assert_eq!(device_label, "Throttle Quadrant");
        assert_ne!(device_label, "Generic HID Joystick");
        assert_eq!(input_label, "X");

        assert_eq!(format(&addr, &cfg), "Throttle Quadrant \u{00b7} X");
    }
}
