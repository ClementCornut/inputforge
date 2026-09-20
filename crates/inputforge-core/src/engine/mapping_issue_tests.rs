use super::*;
use crate::state::{InputIssueKind, InputRole, MappingIssueReason, OutputIssueKind};

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
