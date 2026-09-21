//! Shared per-mapping validation against current input and output capabilities.
use super::validation_inputs::{input_issue, validate_condition, validate_input};
use crate::{
    action::{Action, Mapping},
    output::OutputKind,
    profile::Profile,
    state::{
        AppState, InputIssueKind, InputRole, MappingIssue, MappingIssueReason, OutputIssueKind,
    },
    types::{InputId, OutputId},
};
type Result<T> = std::result::Result<T, MappingIssueReason>;

pub(super) fn issues(state: &AppState) -> Vec<MappingIssue> {
    let Some(profile) = &state.active_profile else {
        return vec![];
    };
    let mut issues = Vec::new();
    for mapping in profile.mappings() {
        if let Err(reason) = validate_mapping(profile, mapping, state) {
            issues.push(issue(mapping, reason));
        }
        for failure in &state.session.output_failures {
            if uses_output(&mapping.actions, failure.output) {
                issues.push(issue(
                    mapping,
                    MappingIssueReason::InjectionFailed {
                        failure: failure.clone(),
                    },
                ));
            }
        }
    }
    issues
}

fn issue(mapping: &Mapping, reason: MappingIssueReason) -> MappingIssue {
    MappingIssue {
        input: mapping.input.clone(),
        mode: mapping.mode.clone(),
        reason,
    }
}

fn uses_output(actions: &[Action], output: OutputKind) -> bool {
    actions.iter().any(|action| match action {
        Action::MapToKeyboard { .. } => output == OutputKind::Keyboard,
        Action::MapToMouse { .. } => output == OutputKind::Mouse,
        Action::Conditional {
            if_true, if_false, ..
        } => uses_output(if_true, output) || uses_output(if_false, output),
        Action::TapGesture {
            single_tap,
            double_tap,
            ..
        } => uses_output(single_tap, output) || uses_output(double_tap, output),
        Action::PressGesture {
            short_press,
            long_press,
            ..
        } => uses_output(short_press, output) || uses_output(long_press, output),
        _ => false,
    })
}

fn validate_mapping(profile: &Profile, mapping: &Mapping, state: &AppState) -> Result<()> {
    validate_input(profile, state, &mapping.input, InputRole::Primary)?;
    validate_actions(profile, mapping, &mapping.actions, state)?;
    crate::profile::validate_mapping_action_tree(&mapping.input, &mapping.actions).map_err(
        |error| MappingIssueReason::InvalidActions {
            details: error.to_string(),
        },
    )
}

fn validate_actions(
    profile: &Profile,
    mapping: &Mapping,
    actions: &[Action],
    state: &AppState,
) -> Result<()> {
    for action in actions {
        match action {
            Action::MapToKeyboard { .. } if !state.session.keyboard_supported => {
                return Err(MappingIssueReason::KeyboardUnavailable);
            }
            Action::MapToMouse { .. } if !state.session.mouse_supported => {
                return Err(MappingIssueReason::MouseUnavailable);
            }
            Action::MapToVJoy { output } => {
                let fail = |problem| MappingIssueReason::Output {
                    address: output.clone(),
                    problem,
                };
                let config = profile
                    .controllers()
                    .and_then(|p| {
                        p.virtual_devices
                            .iter()
                            .find(|c| c.device_id == output.device)
                    })
                    .ok_or_else(|| fail(OutputIssueKind::MissingController))?;
                let compatible = matches!(
                    (mapping.input.input_id(), &output.output),
                    (Some(InputId::Axis { .. }), OutputId::Axis { .. })
                        | (Some(InputId::Button { .. }), OutputId::Button { .. })
                        | (Some(InputId::Hat { .. }), OutputId::Hat { .. })
                );
                if !compatible {
                    return Err(fail(OutputIssueKind::Incompatible));
                }
                let valid = match &output.output {
                    OutputId::Axis { id } => config.axes.contains(id),
                    OutputId::Button { id } => *id > 0 && *id <= config.button_count,
                    OutputId::Hat { id } => *id > 0 && *id <= config.hat_count,
                };
                if !valid {
                    return Err(fail(OutputIssueKind::MissingControl));
                }
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                validate_condition(profile, state, condition)?;
                validate_actions(profile, mapping, if_true, state)?;
                validate_actions(profile, mapping, if_false, state)?;
            }
            Action::TapGesture {
                single_tap,
                double_tap,
                ..
            } => {
                validate_actions(profile, mapping, single_tap, state)?;
                validate_actions(profile, mapping, double_tap, state)?;
            }
            Action::PressGesture {
                short_press,
                long_press,
                ..
            } => {
                validate_actions(profile, mapping, short_press, state)?;
                validate_actions(profile, mapping, long_press, state)?;
            }
            Action::MergeAxis { second_input, .. } => {
                validate_input(profile, state, second_input, InputRole::MergeAxis)?;
                for (address, role) in [
                    (&mapping.input, InputRole::Primary),
                    (second_input, InputRole::MergeAxis),
                ] {
                    if !matches!(address.input_id(), Some(InputId::Axis { .. })) {
                        return Err(input_issue(address, role, InputIssueKind::WrongKind));
                    }
                }
            }
            Action::ChangeMode { strategy } => {
                let (crate::action::ModeChangeStrategy::SwitchTo { mode }
                | crate::action::ModeChangeStrategy::Temporary { mode }) = strategy;
                if !profile.modes().contains(mode) {
                    return Err(MappingIssueReason::MissingMode { mode: mode.clone() });
                }
            }
            _ => {}
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Condition;
    use crate::{
        mode::Modes,
        profile::controllers::{ControllerConfig, DeviceBinding},
        types::*,
    };
    fn validate(p: &Profile) -> Result<()> {
        let mut state = AppState::with_profile(p.clone());
        state.session.monitored = p.controllers().unwrap().selected.clone();
        let found = issues(&state);
        found
            .first()
            .map_or(Ok(()), |issue| Err(issue.reason.clone()))
    }
    #[test]
    fn validates_inactive_branches_foreign_inputs_and_one_based_outputs() {
        let id = DeviceId("evdev:v1:test".into());
        let input = InputAddress::Bound {
            device: id.clone(),
            input: InputId::Button { index: 0 },
        };
        let output = OutputAddress {
            device: 1,
            output: OutputId::Button { id: 2 },
        };
        let mut p = Profile::new(
            "test".into(),
            vec![],
            Modes::new(vec!["Default".into()]).unwrap(),
            vec![],
            vec![],
            "Default".into(),
        );
        p.set_controllers(ControllerConfig {
            legacy_axis_settings: false,
            version: 1,
            selected: vec![id.clone()],
            bindings: vec![DeviceBinding {
                observed_layout: None,
                unavailable: vec![],
                device: id,
                axes: vec![],
                buttons: vec![704],
                hats: vec![],
            }],
            virtual_devices: vec![VirtualDeviceConfig {
                device_id: 1,
                axes: vec![],
                button_count: 2,
                hat_count: 0,
            }],
        })
        .unwrap();
        p.set_mapping(&input, "Default", None, vec![Action::MapToVJoy { output }]);
        validate(&p).unwrap();
        p.set_mapping(
            &input,
            "Default",
            None,
            vec![Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Button { id: 0 },
                },
            }],
        );
        assert!(validate(&p).is_err());
        p.set_mapping(
            &input,
            "Default",
            None,
            vec![Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: InputAddress::Unbound,
                },
                if_true: vec![],
                if_false: vec![],
            }],
        );
        assert!(matches!(
            validate(&p),
            Err(MappingIssueReason::Input {
                role: InputRole::Condition,
                problem: InputIssueKind::Unbound,
                ..
            })
        ));
        for action in [
            Action::Conditional {
                condition: Condition::AxisInRange {
                    input: input.clone(),
                    min: 0.0,
                    max: 1.0,
                },
                if_true: vec![],
                if_false: vec![],
            },
            Action::MergeAxis {
                second_input: input.clone(),
                operation: MergeOp::Average,
            },
        ] {
            p.set_mapping(&input, "Default", None, vec![action]);
            validate(&p).expect_err("a resolvable button cannot stand in for an axis");
        }
    }
}
