//! Batch mapping display labels and option formatting.
use crate::{
    batch_mapping::{RowKind, RowState},
    components::SelectOption,
};
use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis, VirtualDeviceConfig};
pub(super) fn format_virtual_device_option(device: &VirtualDeviceConfig) -> String {
    let axis_count = device.axes.len();
    let axis_suffix = if axis_count == 1 { "axis" } else { "axes" };
    let hat_suffix = if device.hat_count == 1 { "hat" } else { "hats" };

    format!(
        "Device {} · {axis_count} {axis_suffix} · {} btn · {} {hat_suffix}",
        device.device_id, device.button_count, device.hat_count
    )
}

pub(super) fn row_key(row: &RowState) -> String {
    format!("{:?}:{:?}:{}", row.input, row.kind, row.source_index)
}

pub(super) fn source_row_label(kind: RowKind, source_index: u8) -> String {
    let display_index = u16::from(source_index) + 1;
    match kind {
        RowKind::Axis => format!("Axis {display_index}"),
        RowKind::Button => format!("Button {display_index}"),
        RowKind::Hat => format!("Hat {display_index}"),
    }
}

pub(super) fn format_axis_label(axis: VJoyAxis) -> String {
    match axis {
        VJoyAxis::X => "X axis".to_owned(),
        VJoyAxis::Y => "Y axis".to_owned(),
        VJoyAxis::Z => "Z axis".to_owned(),
        VJoyAxis::Rx => "Rx axis".to_owned(),
        VJoyAxis::Ry => "Ry axis".to_owned(),
        VJoyAxis::Rz => "Rz axis".to_owned(),
        VJoyAxis::Slider0 => "Slider 0".to_owned(),
        VJoyAxis::Slider1 => "Slider 1".to_owned(),
    }
}

pub(super) fn format_output_value(address: &OutputAddress) -> String {
    match address.output {
        OutputId::Axis { id } => format!("axis:{id:?}"),
        OutputId::Button { id } => format!("button:{id}"),
        OutputId::Hat { id } => format!("hat:{id}"),
    }
}

pub(super) fn target_options(targets: &[OutputAddress]) -> Vec<SelectOption> {
    std::iter::once(SelectOption {
        value: "(do not map)".into(),
        label: "(do not map)".into(),
        disabled: false,
        class: None,
    })
    .chain(targets.iter().map(|a| SelectOption {
        value: format_output_value(a),
        label: match a.output {
            OutputId::Axis { id } => format_axis_label(id),
            OutputId::Button { id } => format!("Button {id}"),
            OutputId::Hat { id } => format!("Hat {id}"),
        },
        disabled: false,
        class: None,
    }))
    .collect()
}
