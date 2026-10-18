//! Output destination labels shown beside mapping identity.
use inputforge_core::action::Action;
use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};
/// Walk the action tree (DFS pre-order, including `Conditional` branches)
/// and return the formatted label for the first `MapToVJoy` found.
pub(super) fn first_map_to_vjoy_label(actions: &[Action]) -> Option<String> {
    fn walk(actions: &[Action]) -> Option<&OutputAddress> {
        for action in actions {
            match action {
                Action::MapToVJoy { output } => return Some(output),
                Action::Conditional {
                    if_true, if_false, ..
                } => {
                    if let Some(found) = walk(if_true) {
                        return Some(found);
                    }
                    if let Some(found) = walk(if_false) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }
    walk(actions).map(format_output_label)
}
fn format_output_label(output: &OutputAddress) -> String {
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_label_uses_compact_device_address() {
        let output = OutputAddress {
            device: 2,
            output: OutputId::Axis { id: VJoyAxis::X },
        };
        assert_eq!(format_output_label(&output), "Device 2 · X axis");
    }
}
