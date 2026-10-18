//! Source identity, live magnitude and destination labels.
use crate::context::{ConfigSnapshot, LiveSnapshot};
use crate::frame::mapping_list::group::GroupKind;
use inputforge_core::types::{InputAddress, OutputAddress, OutputId, VJoyAxis};
/// Stable u32 mapping for `GroupKind`. The sortable primitive's
/// validator is `fn(&u32, &u32) -> bool`; the consumer's group
/// discriminator passes through that function pointer's signature.
pub(crate) fn group_to_u32(group: GroupKind) -> u32 {
    match group {
        GroupKind::Axes => 0,
        GroupKind::Buttons => 1,
        GroupKind::Hats => 2,
    }
}
/// Live-input magnitude for `addr` in `[0.0, 1.0]`, drives the row's
/// `--row-live-intensity` custom property and gates `is-live-active`.
///
/// Axis intensity is `|value|` in the natural domain (bipolar
/// `[-1, 1]` -> `[0, 1]`; unipolar already `[0, 1]`). Buttons and hats
/// are binary: `1.0` when pressed / off-center, else `0.0`. Returns
/// `0.0` for `Unbound` or when the device or input index is missing
/// from the snapshot.
pub(super) fn live_intensity_for(
    addr: &InputAddress,
    live: &LiveSnapshot,
    _cfg: &ConfigSnapshot,
) -> f64 {
    live.input_readouts
        .get(addr)
        .map_or(0.0, |value| value.intensity)
}
pub(super) fn compact_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X",
            VJoyAxis::Y => "Y",
            VJoyAxis::Z => "Z",
            VJoyAxis::Rx => "Rx",
            VJoyAxis::Ry => "Ry",
            VJoyAxis::Rz => "Rz",
            VJoyAxis::Slider0 => "Slider 0",
            VJoyAxis::Slider1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Btn {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("Device {} · {}", output.device, suffix)
}
/// Encode the full source identity so menu focus returns to the same mounted row.
pub(in crate::frame::mapping_list) fn row_dom_id(mode: &str, input: &InputAddress) -> String {
    use std::fmt::Write as _;
    let mut id = String::from("if-mapping-row-");
    for byte in format!("{mode:?}:{input:?}").bytes() {
        let _ = write!(id, "{byte:02x}");
    }
    id
}
