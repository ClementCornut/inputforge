//! User-facing mapping guidance, separate from engine diagnostics.
use crate::{context::ConfigSnapshot, frame::mapping_list::source_label};
use inputforge_core::{
    output::{OutputFailure, OutputKind, OutputPhase},
    state::{InputIssueKind, InputRole, MappingIssueReason, OutputIssueKind},
    types::{InputAddress, OutputId},
};

pub(super) fn message(reason: &MappingIssueReason, config: &ConfigSnapshot) -> String {
    match reason {
        MappingIssueReason::Input { address, role, problem } => input_message(address, *role, *problem, config),
        MappingIssueReason::KeyboardUnavailable => "Keyboard output isn’t available here. Choose another output.".into(),
        MappingIssueReason::MouseUnavailable => "Mouse output isn’t available here. Choose another output.".into(),
        MappingIssueReason::InjectionFailed { failure } => output_failure_message(failure),
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

pub(super) fn output_failure_message(failure: &OutputFailure) -> String {
    let output = match failure.output {
        OutputKind::Keyboard => "Keyboard",
        OutputKind::Mouse => "Mouse",
    };
    match (failure.phase, failure.category) {
        (OutputPhase::Initialization, std::io::ErrorKind::PermissionDenied) => format!(
            "{output} output could not start. Check `/dev/uinput` access for your user, then Retry."
        ),
        (OutputPhase::Initialization, std::io::ErrorKind::NotFound) => format!(
            "{output} output could not start because `/dev/uinput` is unavailable. Restore uinput availability, then Retry."
        ),
        (OutputPhase::Readiness, std::io::ErrorKind::InvalidData) => format!(
            "{output} output could not be verified. Review the technical details, then Retry."
        ),
        (OutputPhase::Readiness, _) => format!(
            "{output} output was created but its system metadata is not ready. Check sysfs and udev availability, then Retry."
        ),
        (OutputPhase::Emission, _) => format!(
            "{output} output failed and routing stopped. Review the details, restore output access, then Retry."
        ),
        (OutputPhase::Release, _) => format!(
            "{output} output could not be fully released. Routing stopped and its virtual device was closed. Review the details before Retry."
        ),
        (OutputPhase::Initialization, _) => format!(
            "{output} output could not start. Review the technical details, restore output access, then Retry."
        ),
    }
}

pub(super) fn technical_details(failure: &OutputFailure) -> String {
    use std::fmt::Write as _;
    let mut details = failure.details.clone();
    for cleanup in &failure.cleanup {
        let _ = write!(details, "\ncleanup: {cleanup}");
    }
    details
}

pub(super) fn cleanup_notice(failure: &OutputFailure) -> Option<&'static str> {
    (!failure.cleanup.is_empty())
        .then_some("Cleanup also failed. Review the technical details before retrying.")
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
    use std::io;

    fn injection(
        output: OutputKind,
        phase: OutputPhase,
        category: io::ErrorKind,
        cleanup: &[&str],
    ) -> MappingIssueReason {
        MappingIssueReason::InjectionFailed {
            failure: OutputFailure {
                output,
                phase,
                category,
                details: "opaque native details".into(),
                cleanup: cleanup.iter().map(|detail| (*detail).into()).collect(),
            },
        }
    }

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

    #[test]
    fn injection_messages_cover_every_approved_failure_class() {
        let config = ConfigSnapshot::default();
        let cases = [
            (
                injection(
                    OutputKind::Keyboard,
                    OutputPhase::Initialization,
                    io::ErrorKind::PermissionDenied,
                    &[],
                ),
                "Keyboard output could not start. Check `/dev/uinput` access for your user, then Retry.",
            ),
            (
                injection(
                    OutputKind::Keyboard,
                    OutputPhase::Initialization,
                    io::ErrorKind::NotFound,
                    &[],
                ),
                "Keyboard output could not start because `/dev/uinput` is unavailable. Restore uinput availability, then Retry.",
            ),
            (
                injection(
                    OutputKind::Mouse,
                    OutputPhase::Readiness,
                    io::ErrorKind::PermissionDenied,
                    &[],
                ),
                "Mouse output was created but its system metadata is not ready. Check sysfs and udev availability, then Retry.",
            ),
            (
                injection(
                    OutputKind::Mouse,
                    OutputPhase::Readiness,
                    io::ErrorKind::InvalidData,
                    &[],
                ),
                "Mouse output could not be verified. Review the technical details, then Retry.",
            ),
            (
                injection(
                    OutputKind::Keyboard,
                    OutputPhase::Emission,
                    io::ErrorKind::BrokenPipe,
                    &[],
                ),
                "Keyboard output failed and routing stopped. Review the details, restore output access, then Retry.",
            ),
            (
                injection(
                    OutputKind::Keyboard,
                    OutputPhase::Release,
                    io::ErrorKind::BrokenPipe,
                    &["secondary cleanup"],
                ),
                "Keyboard output could not be fully released. Routing stopped and its virtual device was closed. Review the details before Retry.",
            ),
        ];

        for (reason, expected) in cases {
            assert_eq!(message(&reason, &config), expected);
        }
    }

    #[test]
    fn injection_technical_details_keep_cleanup_separate_from_default_copy() {
        let reason = injection(
            OutputKind::Keyboard,
            OutputPhase::Release,
            io::ErrorKind::BrokenPipe,
            &["secondary cleanup"],
        );
        assert!(!message(&reason, &ConfigSnapshot::default()).contains("opaque"));
        let MappingIssueReason::InjectionFailed { failure } = reason else {
            unreachable!();
        };
        assert!(technical_details(&failure).contains("opaque native details"));
        assert!(technical_details(&failure).contains("secondary cleanup"));
        assert!(cleanup_notice(&failure).is_some());
    }
}
