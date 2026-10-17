//! Formatting and constant-time access to prepared readout values.
use crate::context::{ConfigSnapshot, LiveSnapshot};
pub(crate) use crate::readouts::{AxisDisplay, ReadoutDisplay};
use inputforge_core::types::{
    AxisPolarity, HatDirection, InputAddress, OutputAddress, OutputId, VJoyAxis,
};
pub(crate) fn read_input_display(
    addr: &InputAddress,
    live: &LiveSnapshot,
    _cfg: &ConfigSnapshot,
) -> ReadoutDisplay {
    live.input_readouts.get(addr).map_or_default(|v| v.display)
}
pub(crate) fn read_axis_display(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> AxisDisplay {
    match read_input_display(addr, live, cfg) {
        ReadoutDisplay::Axis(value) => value,
        _ => AxisDisplay {
            value: 0.0,
            polarity: AxisPolarity::Bipolar,
        },
    }
}
pub(crate) fn read_button_pressed(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> bool {
    matches!(
        read_input_display(addr, live, cfg),
        ReadoutDisplay::Button { pressed: true }
    )
}
pub(crate) fn read_hat_direction(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> HatDirection {
    match read_input_display(addr, live, cfg) {
        ReadoutDisplay::Hat { direction } => direction,
        _ => HatDirection::Center,
    }
}
pub(super) fn hat_direction_label(direction: HatDirection) -> &'static str {
    match direction {
        HatDirection::Center => "Center",
        HatDirection::N => "N",
        HatDirection::NE => "NE",
        HatDirection::E => "E",
        HatDirection::SE => "SE",
        HatDirection::S => "S",
        HatDirection::SW => "SW",
        HatDirection::W => "W",
        HatDirection::NW => "NW",
    }
}

pub(super) fn hat_glyph_for(direction: HatDirection) -> char {
    match direction {
        HatDirection::N => '\u{2191}',
        HatDirection::NE => '\u{2197}',
        HatDirection::E => '\u{2192}',
        HatDirection::SE => '\u{2198}',
        HatDirection::S => '\u{2193}',
        HatDirection::SW => '\u{2199}',
        HatDirection::W => '\u{2190}',
        HatDirection::NW => '\u{2196}',
        HatDirection::Center => '\u{00b7}',
    }
}

/// Format a `KeyCombo` as `Ctrl + Shift + Space`.
///
/// Modifiers keep their configured order, followed by the key name.
pub(super) fn format_key_combo(combo: &inputforge_core::types::KeyCombo) -> String {
    let mut parts: Vec<String> = combo
        .modifiers
        .iter()
        .map(|modifier| modifier.display_label())
        .map(str::to_owned)
        .collect();
    parts.push(combo.key.display_label().into_owned());
    parts.join(" + ")
}

/// Format a virtual output address as `Device <device> \u{00b7} <axis|button|hat>`.
pub(super) fn format_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X axis",
            VJoyAxis::Y => "Y axis",
            VJoyAxis::Z => "Z axis",
            VJoyAxis::Rx => "Rx axis",
            VJoyAxis::Ry => "Ry axis",
            VJoyAxis::Rz => "Rz axis",
            VJoyAxis::Slider0 => "Slider 0",
            VJoyAxis::Slider1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Button {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("Device {} \u{00b7} {}", output.device, suffix)
}

/// Format a percentage string for the readout label.
///
/// Bipolar axes show a sign prefix (`+0.00` / `-0.00`) so the center is
/// unambiguous. Unipolar axes omit the sign. Sub-precision noise rounds
/// to a literal `0.0` so idle is always `0.00` / `+0.00`.
pub(super) fn format_percentage(display: &AxisDisplay) -> String {
    let value = if display.value.abs() < 0.005 {
        0.0
    } else {
        display.value
    };
    match display.polarity {
        AxisPolarity::Bipolar => format!("{value:+.2}"),
        AxisPolarity::Unipolar => format!("{value:.2}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn format_percentage_bipolar_includes_sign() {
        let d = AxisDisplay {
            value: 0.5,
            polarity: AxisPolarity::Bipolar,
        };
        assert_eq!(format_percentage(&d), "+0.50");
    }

    #[test]
    fn format_percentage_unipolar_omits_sign() {
        let d = AxisDisplay {
            value: 0.25,
            polarity: AxisPolarity::Unipolar,
        };
        assert_eq!(format_percentage(&d), "0.25");
    }

    #[test]
    fn format_output_label_axis() {
        let out = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::Y },
        };
        assert_eq!(format_output_label(&out), "Device 1 \u{00b7} Y axis");
    }
}
