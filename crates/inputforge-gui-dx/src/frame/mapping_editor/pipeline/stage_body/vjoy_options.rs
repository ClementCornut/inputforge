//! Virtual-output display labels and available option values.
use inputforge_core::types::{OutputId, VJoyAxis};
/// Human-readable label for a [`VJoyAxis`] variant.
///
/// Matches the canonical labels used in the egui UI (see `vjoy.rs`).
pub(super) fn axis_label(axis: VJoyAxis) -> &'static str {
    match axis {
        VJoyAxis::X => "X axis",
        VJoyAxis::Y => "Y axis",
        VJoyAxis::Z => "Z axis",
        VJoyAxis::Rx => "Rx axis",
        VJoyAxis::Ry => "Ry axis",
        VJoyAxis::Rz => "Rz axis",
        VJoyAxis::Slider0 => "Slider 0",
        VJoyAxis::Slider1 => "Slider 1",
    }
}

/// Stable string key for an [`OutputId`] used as a `Select` option value.
pub(super) fn output_id_key(id: &OutputId) -> String {
    match id {
        OutputId::Axis { id: axis } => format!("axis:{}", axis_label(*axis)),
        OutputId::Button { id: n } => format!("button:{n}"),
        OutputId::Hat { id: n } => format!("hat:{n}"),
    }
}

/// Parse a `Select` option value back into an [`OutputId`]. Returns `None`
/// when the key string does not match any known pattern (should not occur
/// under normal operation).
pub(super) fn available_outputs(
    config: &inputforge_core::types::VirtualDeviceConfig,
) -> Vec<OutputId> {
    config
        .axes
        .iter()
        .map(|id| OutputId::Axis { id: *id })
        .chain((1..=config.button_count).map(|id| OutputId::Button { id }))
        .chain((1..=config.hat_count).map(|id| OutputId::Hat { id }))
        .collect()
}
