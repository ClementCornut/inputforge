//! Select typed destination values and observed activity.
use inputforge_core::{
    processing::into_natural_domain,
    types::{AxisPolarity, HatDirection, OutputAddress, OutputId},
};
use inputforge_gui_dx::{
    readouts::{AxisDisplay, ReadoutDisplay},
    snapshots::{ConfigSnapshot, LiveSnapshot},
};
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
