use super::*;
use crate::state::{InputIssueKind, InputRole, MappingIssueReason, OutputIssueKind};

fn failed(output: OutputKind) -> OutputFailure {
    OutputFailure {
        output,
        phase: OutputPhase::Emission,
        category: std::io::ErrorKind::BrokenPipe,
        details: format!("{output} failed"),
        cleanup: vec![],
    }
}

#[test]
fn input_diagnostics_distinguish_selection_connection_layout_and_nested_dependencies() {
    let (e, _tx, _s, _temp) = harness();
    let mut state = e.state.write();
    let address = button(false).source;
    let id = address.device().unwrap().clone();
    assert!(matches!(
        &validation::issues(&state)[0].reason,
        MappingIssueReason::Input {
            problem: InputIssueKind::Disconnected,
            role: InputRole::Primary,
            ..
        }
    ));
    state.session.monitored.push(id.clone());
    let mut config = state
        .active_profile
        .as_ref()
        .unwrap()
        .controllers()
        .unwrap()
        .clone();
    config.selected.clear();
    state
        .active_profile
        .as_mut()
        .unwrap()
        .set_controllers(config.clone())
        .unwrap();
    assert!(matches!(
        &validation::issues(&state)[0].reason,
        MappingIssueReason::Input {
            problem: InputIssueKind::Unselected,
            ..
        }
    ));
    config.selected.push(id);
    config.bindings[0]
        .unavailable
        .push(InputId::Button { index: 0 });
    state
        .active_profile
        .as_mut()
        .unwrap()
        .set_controllers(config.clone())
        .unwrap();
    assert!(matches!(
        &validation::issues(&state)[0].reason,
        MappingIssueReason::Input {
            problem: InputIssueKind::ChangedControl,
            ..
        }
    ));
    config.bindings[0].unavailable.clear();
    state
        .active_profile
        .as_mut()
        .unwrap()
        .set_controllers(config)
        .unwrap();
    for (action, role) in [
        (
            Action::MergeAxis {
                second_input: InputAddress::Unbound,
                operation: MergeOp::Average,
            },
            InputRole::MergeAxis,
        ),
        (
            Action::Conditional {
                condition: crate::action::Condition::Not {
                    condition: Box::new(crate::action::Condition::All {
                        conditions: vec![crate::action::Condition::ButtonPressed {
                            input: InputAddress::Unbound,
                        }],
                    }),
                },
                if_true: vec![],
                if_false: vec![],
            },
            InputRole::Condition,
        ),
    ] {
        state
            .active_profile
            .as_mut()
            .unwrap()
            .set_mapping(&address, "Default", None, vec![action]);
        assert!(matches!(&validation::issues(&state)[0].reason,
            MappingIssueReason::Input { problem: InputIssueKind::Unbound, role: actual, .. } if *actual == role));
    }
}

#[test]
fn inactive_nested_outputs_report_the_affected_destination() {
    let (e, _tx, _s, _temp) = harness();
    let mut state = e.state.write();
    state
        .session
        .monitored
        .push(button(false).source.device().unwrap().clone());
    let output = OutputAddress {
        device: 1,
        output: OutputId::Button { id: 9 },
    };
    state.active_profile.as_mut().unwrap().set_mapping(
        &button(false).source,
        "Default",
        None,
        vec![Action::TapGesture {
            threshold_ms: 50,
            fire_single_immediately: false,
            single_tap: vec![],
            double_tap: vec![Action::MapToVJoy {
                output: output.clone(),
            }],
        }],
    );
    assert_eq!(
        validation::issues(&state)[0].reason,
        MappingIssueReason::Output {
            address: output,
            problem: OutputIssueKind::MissingControl
        }
    );
}

#[test]
fn injection_structural_and_output_failures_coexist_for_one_mapping() {
    let (engine, _tx, _script, _temp) = harness();
    let mut state = engine.state.write();
    let input = button(false).source;
    state
        .session
        .monitored
        .push(input.device().unwrap().clone());
    state.session.output_failures = vec![failed(OutputKind::Keyboard)];
    state.active_profile.as_mut().unwrap().set_mapping(
        &input,
        "Default",
        None,
        vec![
            Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Button { id: 9 },
                },
            },
            Action::MapToKeyboard {
                key: KeyCombo {
                    key: PhysicalKey::KeyA,
                    modifiers: vec![],
                },
                behavior: crate::action::OutputBehavior::Hold,
            },
        ],
    );

    let issues = validation::issues(&state);
    assert!(issues.iter().any(|issue| matches!(
        issue.reason,
        MappingIssueReason::Output {
            problem: OutputIssueKind::MissingControl,
            ..
        }
    )));
    assert!(issues.iter().any(|issue| matches!(
        &issue.reason,
        MappingIssueReason::InjectionFailed { failure }
            if failure.output == OutputKind::Keyboard
    )));
}

#[test]
fn injection_nested_inactive_actions_attach_once_per_failed_sink() {
    let (engine, _tx, _script, _temp) = harness();
    let mut state = engine.state.write();
    let input = button(false).source;
    state
        .session
        .monitored
        .push(input.device().unwrap().clone());
    state.session.output_failures = vec![failed(OutputKind::Keyboard), failed(OutputKind::Mouse)];
    let keyboard = Action::MapToKeyboard {
        key: KeyCombo {
            key: PhysicalKey::KeyA,
            modifiers: vec![],
        },
        behavior: crate::action::OutputBehavior::Hold,
    };
    let mouse = Action::MapToMouse {
        target: crate::action::MouseTarget::LeftButton,
        behavior: crate::action::OutputBehavior::Hold,
    };
    state.active_profile.as_mut().unwrap().set_mapping(
        &input,
        "Default",
        None,
        vec![Action::Conditional {
            condition: crate::action::Condition::ButtonPressed {
                input: input.clone(),
            },
            if_true: vec![Action::TapGesture {
                threshold_ms: 50,
                fire_single_immediately: false,
                single_tap: vec![keyboard.clone(), keyboard],
                double_tap: vec![],
            }],
            if_false: vec![Action::PressGesture {
                threshold_ms: 50,
                fire_long_when_threshold_crossed: false,
                short_press: vec![],
                long_press: vec![mouse],
            }],
        }],
    );

    let failures: Vec<_> = validation::issues(&state)
        .into_iter()
        .filter_map(|issue| match issue.reason {
            MappingIssueReason::InjectionFailed { failure } => Some(failure.output),
            _ => None,
        })
        .collect();
    assert_eq!(failures, [OutputKind::Keyboard, OutputKind::Mouse]);
}

#[test]
fn injection_unrelated_mappings_and_empty_profiles_get_no_mapping_issue() {
    let (engine, _tx, _script, _temp) = harness();
    let mut state = engine.state.write();
    let input = button(false).source;
    state
        .session
        .monitored
        .push(input.device().unwrap().clone());
    state.session.output_failures = vec![failed(OutputKind::Keyboard)];
    assert!(
        !validation::issues(&state)
            .iter()
            .any(|issue| matches!(issue.reason, MappingIssueReason::InjectionFailed { .. }))
    );

    state.active_profile = Some(Profile::new(
        "empty".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    assert!(validation::issues(&state).is_empty());
    assert_eq!(state.session.output_failures.len(), 1);
}
