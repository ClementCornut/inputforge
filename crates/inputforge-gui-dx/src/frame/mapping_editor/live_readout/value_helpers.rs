// Rust guideline compliant 2026-05-03

//! Value-level helpers shared by IN, OUT, and chain rows: axis-display
//! conversion, percentage formatting, output-label formatting, and the
//! merge-polarity inference table. No Dioxus surface; pure functions on
//! the snapshot types.

use inputforge_core::processing::into_natural_domain;
use inputforge_core::types::{
    AxisPolarity, HatDirection, InputAddress, InputId, InputValue, MergeOp, OutputAddress,
    OutputId, VJoyAxis,
};

use crate::context::{ConfigSnapshot, LiveSnapshot};

/// Thin display value carried through the readout component tree.
///
/// `value` is normalized to the polarity's natural domain:
/// - `Bipolar`: `[-1.0, 1.0]`, where 0 is centered.
/// - `Unipolar`: `[0.0, 1.0]`, where 0 is idle and 1 is fully pressed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct AxisDisplay {
    pub value: f64,
    pub polarity: AxisPolarity,
}

/// Typed display value carried by top-level live readout rows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum ReadoutDisplay {
    Axis(AxisDisplay),
    Button { pressed: bool },
    Hat { direction: HatDirection },
}

/// Read the typed input value for `addr` from the live snapshot.
pub(crate) fn read_input_display(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> ReadoutDisplay {
    match addr.input_id() {
        Some(InputId::Axis { .. }) => ReadoutDisplay::Axis(read_axis_display(addr, live, cfg)),
        Some(InputId::Button { .. }) => ReadoutDisplay::Button {
            pressed: read_button_pressed(addr, live, cfg),
        },
        Some(InputId::Hat { .. }) => ReadoutDisplay::Hat {
            direction: read_hat_direction(addr, live, cfg),
        },
        None => ReadoutDisplay::Axis(AxisDisplay {
            value: 0.0,
            polarity: AxisPolarity::Bipolar,
        }),
    }
}

/// Read the raw axis value and polarity for `addr` from the live snapshot.
///
/// Falls back to `(0.0, Bipolar)` when the device or axis index is not
/// present in the snapshot.
pub(crate) fn read_axis_display(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> AxisDisplay {
    let Some(InputId::Axis { index }) = addr.input_id() else {
        return AxisDisplay {
            value: 0.0,
            polarity: AxisPolarity::Bipolar,
        };
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    if let Some(di) = dev_idx
        && let Some(dev_inputs) = live.device_inputs.get(di)
        && let Some(&(raw, polarity)) = dev_inputs.axes.get(usize::from(*index))
    {
        return AxisDisplay {
            value: into_natural_domain(raw, polarity),
            polarity,
        };
    }
    AxisDisplay {
        value: 0.0,
        polarity: AxisPolarity::Bipolar,
    }
}

/// Read whether the button at `addr` is currently pressed.
///
/// Returns `false` when the address is not a button, or when the device
/// or input index is absent from the live snapshot.
pub(crate) fn read_button_pressed(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> bool {
    let Some(InputId::Button { index }) = addr.input_id() else {
        return false;
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    dev_idx
        .and_then(|di| live.device_inputs.get(di))
        .and_then(|dev_inputs| dev_inputs.buttons.get(usize::from(*index)).copied())
        .unwrap_or(false)
}

/// Read the hat direction at `addr` from the live snapshot.
///
/// Returns `Center` when the address is not a hat, or when the device
/// or input index is absent from the live snapshot.
pub(crate) fn read_hat_direction(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> HatDirection {
    let Some(InputId::Hat { index }) = addr.input_id() else {
        return HatDirection::Center;
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    dev_idx
        .and_then(|di| live.device_inputs.get(di))
        .and_then(|dev_inputs| dev_inputs.hats.get(usize::from(*index)).copied())
        .unwrap_or(HatDirection::Center)
}

/// Read the engine output value for `out` from the live snapshot.
///
/// Mirrors `read_axis_display` but indexes into `live.output_values`.
/// `polarity` is the inferred output polarity. Falls back to `0.0` when
/// the device or output id is absent.
pub(super) fn read_output_display(
    out: &OutputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
    polarity: AxisPolarity,
) -> AxisDisplay {
    let dev_idx = cfg
        .virtual_devices
        .iter()
        .position(|v| v.device_id == out.device);
    let raw = dev_idx
        .and_then(|di| live.output_values.get(di))
        .and_then(|vals| match out.output {
            OutputId::Axis { id } => vals
                .axes
                .iter()
                .find_map(|&(axis, value)| (axis == id).then_some(value)),
            OutputId::Button { id } => {
                let idx = usize::from(id.checked_sub(1)?);
                vals.buttons.get(idx).map(|&b| if b { 1.0 } else { 0.0 })
            }
            OutputId::Hat { .. } => None,
        })
        .unwrap_or(0.0);
    AxisDisplay {
        value: into_natural_domain(raw, polarity),
        polarity,
    }
}

/// Read whether a vJoy button output is currently pressed.
///
/// Returns `false` for missing entries.
pub(super) fn read_output_button(
    out: &OutputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> bool {
    let OutputId::Button { id } = out.output else {
        return false;
    };
    let Some(idx) = id.checked_sub(1) else {
        return false;
    };
    cfg.virtual_devices
        .iter()
        .position(|v| v.device_id == out.device)
        .and_then(|di| live.output_values.get(di))
        .and_then(|vals| vals.buttons.get(usize::from(idx)).copied())
        .unwrap_or(false)
}

/// Read the current direction emitted to a vJoy hat output.
pub(super) fn read_output_hat(
    out: &OutputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> HatDirection {
    let OutputId::Hat { id } = out.output else {
        return HatDirection::Center;
    };
    let Some(idx) = id.checked_sub(1) else {
        return HatDirection::Center;
    };
    cfg.virtual_devices
        .iter()
        .position(|v| v.device_id == out.device)
        .and_then(|di| live.output_values.get(di))
        .and_then(|vals| vals.hats.get(usize::from(idx)).copied())
        .unwrap_or(HatDirection::Center)
}

/// Read the typed output value for a vJoy destination from the live snapshot.
pub(super) fn read_output_typed_display(
    out: &OutputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
    polarity: AxisPolarity,
) -> ReadoutDisplay {
    match out.output {
        OutputId::Axis { .. } => {
            ReadoutDisplay::Axis(read_output_display(out, live, cfg, polarity))
        }
        OutputId::Button { .. } => ReadoutDisplay::Button {
            pressed: read_output_button(out, live, cfg),
        },
        OutputId::Hat { .. } => ReadoutDisplay::Hat {
            direction: read_output_hat(out, live, cfg),
        },
    }
}

/// Extract a scalar f64 from any `InputValue`.
pub(super) fn axis_f64(v: &InputValue) -> f64 {
    match v {
        InputValue::Axis { value, .. } => value.value(),
        InputValue::Button { pressed } => {
            if *pressed {
                1.0
            } else {
                0.0
            }
        }
        InputValue::Hat { .. } => 0.0,
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
    use inputforge_core::types::KeyModifier;
    let mut parts: Vec<String> = combo
        .modifiers
        .iter()
        .map(|m| match m {
            KeyModifier::Ctrl => "Ctrl",
            KeyModifier::Shift => "Shift",
            KeyModifier::Alt => "Alt",
            KeyModifier::Win => "Win",
        })
        .map(str::to_owned)
        .collect();
    parts.push(combo.key.display_label().into_owned());
    parts.join(" + ")
}

/// Format a vJoy output address as `vJoy <device> \u{00b7} <axis|button|hat>`.
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
    format!("vJoy {} \u{00b7} {}", output.device, suffix)
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

/// Infer the natural polarity of a merge result from the operator and
/// each input's polarity.
#[must_use]
pub(super) fn merge_output_polarity(
    op: MergeOp,
    primary: AxisPolarity,
    secondary: AxisPolarity,
) -> AxisPolarity {
    match op {
        MergeOp::Bidirectional => AxisPolarity::Bipolar,
        MergeOp::Average | MergeOp::Maximum => {
            if primary == secondary {
                primary
            } else {
                AxisPolarity::Bipolar
            }
        }
    }
}

/// Fold merge operations along one output path to infer terminal polarity.
#[must_use]
pub(super) fn infer_output_polarity(
    primary_polarity: AxisPolarity,
    merges_on_path: &[(MergeOp, AxisPolarity)],
) -> AxisPolarity {
    merges_on_path
        .iter()
        .fold(primary_polarity, |acc, (op, secondary)| {
            merge_output_polarity(*op, acc, *secondary)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::VjoyOutputValues;
    use inputforge_core::types::VirtualDeviceConfig;

    #[test]
    fn read_output_typed_display_reads_one_based_hat_output() {
        let cfg = ConfigSnapshot {
            virtual_devices: vec![VirtualDeviceConfig {
                device_id: 1,
                axes: vec![],
                button_count: 0,
                hat_count: 1,
            }],
            ..ConfigSnapshot::default()
        };
        let live = LiveSnapshot {
            device_inputs: vec![],
            output_values: vec![VjoyOutputValues {
                axes: vec![],
                buttons: vec![],
                hats: vec![HatDirection::SE],
            }],
        };
        let out = OutputAddress {
            device: 1,
            output: OutputId::Hat { id: 1 },
        };

        assert_eq!(
            read_output_typed_display(&out, &live, &cfg, AxisPolarity::Bipolar),
            ReadoutDisplay::Hat {
                direction: HatDirection::SE,
            }
        );
    }

    #[test]
    fn merge_output_polarity_bidirectional_always_bipolar() {
        for primary in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
            for secondary in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                assert_eq!(
                    merge_output_polarity(MergeOp::Bidirectional, primary, secondary),
                    AxisPolarity::Bipolar
                );
            }
        }
    }

    #[test]
    fn merge_output_polarity_average_uu_is_unipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Unipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn merge_output_polarity_average_bb_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Bipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn merge_output_polarity_average_mixed_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Bipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Bipolar
        );
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Unipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn merge_output_polarity_maximum_uu_is_unipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Unipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn merge_output_polarity_maximum_bb_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Bipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn merge_output_polarity_maximum_mixed_is_bipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Bipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Bipolar
        );
        assert_eq!(
            merge_output_polarity(
                MergeOp::Maximum,
                AxisPolarity::Unipolar,
                AxisPolarity::Bipolar
            ),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn merge_output_polarity_average_and_maximum_are_commutative() {
        for op in [MergeOp::Average, MergeOp::Maximum] {
            for a in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                for b in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                    assert_eq!(
                        merge_output_polarity(op, a, b),
                        merge_output_polarity(op, b, a),
                        "{op:?}({a:?}, {b:?}) should equal {op:?}({b:?}, {a:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn infer_output_polarity_no_merges_inherits_primary() {
        assert_eq!(
            infer_output_polarity(AxisPolarity::Unipolar, &[]),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn infer_output_polarity_chained_merges_compose_left_to_right() {
        // Unipolar primary plus Bidirectional with Unipolar gives Bipolar.
        // Then Average with Unipolar keeps Bipolar because mixed inputs promote.
        let path = [
            (MergeOp::Bidirectional, AxisPolarity::Unipolar),
            (MergeOp::Average, AxisPolarity::Unipolar),
        ];
        assert_eq!(
            infer_output_polarity(AxisPolarity::Unipolar, &path),
            AxisPolarity::Bipolar
        );
    }

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
        assert_eq!(format_output_label(&out), "vJoy 1 \u{00b7} Y axis");
    }
}
