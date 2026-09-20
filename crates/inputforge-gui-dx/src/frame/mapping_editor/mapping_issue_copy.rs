//! User-facing mapping guidance, separate from engine diagnostics.
use crate::{context::ConfigSnapshot, frame::mapping_list::source_label};
use inputforge_core::{
    state::{InputIssueKind, InputRole, MappingIssueReason, OutputIssueKind},
    types::{InputAddress, OutputId},
};

pub(super) fn message(reason: &MappingIssueReason, config: &ConfigSnapshot) -> String {
    match reason {
        MappingIssueReason::Input { address, role, problem } => input_message(address, *role, *problem, config),
        MappingIssueReason::KeyboardUnavailable => "Keyboard output isn’t available here. Choose another output.".into(),
        MappingIssueReason::MouseUnavailable => "Mouse output isn’t available here. Choose another output.".into(),
        MappingIssueReason::MissingMode { mode } => format!("Mode ‘{mode}’ no longer exists. Choose an available mode in the mode-change action."),
        MappingIssueReason::InvalidCondition { input } => format!("The range for {} is invalid. Set its minimum at or below its maximum.", input_label(input, config)),
        MappingIssueReason::InvalidActions { .. } => "This mapping contains an incompatible action. Check its gesture and processing steps; technical details are below.".into(),
        MappingIssueReason::Output { address, problem } => {
            let slot = address.device;
            let control = match address.output {
                OutputId::Axis { id } => format!("Axis {id:?}"),
                OutputId::Button { id } => format!("Button {id}"),
                OutputId::Hat { id } => format!("Hat {id}"),
            };
            match problem {
                OutputIssueKind::MissingController => format!("Virtual controller {slot} isn’t configured. Add it in Devices or choose another output."),
                OutputIssueKind::MissingControl => format!("Virtual controller {slot} has no {control}. Choose an available output or change its layout in Devices."),
                OutputIssueKind::Incompatible => format!("{control} on virtual controller {slot} doesn’t match this input type. Choose a compatible output."),
            }
        }
    }
}

fn input_label(address: &InputAddress, config: &ConfigSnapshot) -> String {
    let (_, control) = source_label::split_label(address, config);
    format!("‘{} · {control}’", device_label(address, config))
}

fn device_label(address: &InputAddress, config: &ConfigSnapshot) -> String {
    address
        .device()
        .filter(|id| config.device_display_names.contains_key(*id))
        .map_or_else(
            || "Unknown controller".to_owned(),
            |id| config.device_display_name(id),
        )
}

fn input_message(
    address: &InputAddress,
    role: InputRole,
    problem: InputIssueKind,
    config: &ConfigSnapshot,
) -> String {
    let location = match role {
        InputRole::Primary => "the mapping’s input",
        InputRole::MergeAxis => "the merge’s second axis",
        InputRole::Condition => "the condition’s input",
    };
    let device = device_label(address, config);
    match problem {
        InputIssueKind::Unbound => format!("Choose {location} using Rebind."),
        InputIssueKind::Unselected => format!(
            "Select ‘{device}’ in Devices to use {location}. Stop routing first if it is running."
        ),
        InputIssueKind::Disconnected => format!("Connect ‘{device}’ to use {location}."),
        InputIssueKind::NotReady => format!(
            "‘{device}’ is connected but cannot be read yet. Check its status in Devices, then refresh devices."
        ),
        InputIssueKind::MissingControl => format!(
            "{} is unavailable. Use Rebind for {location} to choose an available control.",
            input_label(address, config)
        ),
        InputIssueKind::ChangedControl => format!(
            "{} is missing or changed after a controller layout change. Use Rebind for {location} to confirm the control.",
            input_label(address, config)
        ),
        InputIssueKind::WrongKind => format!(
            "{} has the wrong control type. Use Rebind to choose a compatible control for {location}.",
            input_label(address, config)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId};

    #[test]
    fn input_errors_use_aliases_and_never_expose_unknown_native_ids() {
        let id = DeviceId("evdev:v1:private-native-identifier".into());
        let address = InputAddress::Bound {
            device: id.clone(),
            input: InputId::Button { index: 0 },
        };
        let reason = MappingIssueReason::Input {
            address,
            role: InputRole::Condition,
            problem: InputIssueKind::Unselected,
        };
        let mut config = ConfigSnapshot::default();
        assert!(!message(&reason, &config).contains(&id.0));
        config.device_display_names.insert(id, "My pedals".into());
        let text = message(&reason, &config);
        assert!(text.contains("Select ‘My pedals’ in Devices"));
        assert!(text.contains("condition’s input"));
        assert!(!text.contains("evdev:"));
    }
}
