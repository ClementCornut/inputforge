// Rust guideline compliant 2026-05-04

//! Input dependency discovery for engine event routing.

use crate::action::{Action, Condition, Mapping};
use crate::mode::{ModeTree, resolve_mapping};
use crate::types::InputAddress;

/// Return active mappings whose primary or derived inputs depend on `source`.
pub(super) fn active_mappings_for_event<'a>(
    mappings: &'a [Mapping],
    source: &InputAddress,
    mode: &str,
    tree: &ModeTree,
) -> Vec<&'a Mapping> {
    let mut out = Vec::new();

    if let Some(primary) = resolve_mapping(mappings, source, mode, tree) {
        out.push(primary);
    }

    for mapping in mappings {
        if out.iter().any(|seen| std::ptr::eq(*seen, mapping)) {
            continue;
        }
        if !mapping_dependencies(mapping)
            .iter()
            .any(|dep| dep == source)
        {
            continue;
        }
        let Some(active) = resolve_mapping(mappings, &mapping.input, mode, tree) else {
            continue;
        };
        if std::ptr::eq(active, mapping) {
            out.push(mapping);
        }
    }

    out
}

/// Return all bound inputs that can affect `mapping`.
pub(super) fn mapping_dependencies(mapping: &Mapping) -> Vec<InputAddress> {
    let mut out = Vec::new();
    push_dependency(&mut out, &mapping.input);
    collect_action_dependencies(&mapping.actions, &mut out);
    out
}

fn collect_action_dependencies(actions: &[Action], out: &mut Vec<InputAddress>) {
    for action in actions {
        match action {
            Action::MergeAxis { second_input, .. } => {
                push_dependency(out, second_input);
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                collect_condition_dependencies(condition, out);
                collect_action_dependencies(if_true, out);
                collect_action_dependencies(if_false, out);
            }
            Action::ResponseCurve { .. }
            | Action::Deadzone { .. }
            | Action::Invert
            | Action::MapToVJoy { .. }
            | Action::MapToKeyboard { .. }
            | Action::ChangeMode { .. } => {}
        }
    }
}

fn collect_condition_dependencies(condition: &Condition, out: &mut Vec<InputAddress>) {
    match condition {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => push_dependency(out, input),
        Condition::All { conditions } | Condition::Any { conditions } => {
            for inner in conditions {
                collect_condition_dependencies(inner, out);
            }
        }
        Condition::Not { condition } => collect_condition_dependencies(condition, out),
    }
}

fn push_dependency(out: &mut Vec<InputAddress>, input: &InputAddress) {
    if input.is_unbound() || out.iter().any(|seen| seen == input) {
        return;
    }
    out.push(input.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, InputId, MergeOp, OutputAddress, OutputId, VJoyAxis};

    fn axis(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index },
        }
    }

    fn button(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index },
        }
    }

    fn output() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    fn mapping(actions: Vec<Action>) -> Mapping {
        Mapping {
            input: axis(0),
            mode: "Default".to_owned(),
            name: None,
            actions,
        }
    }

    #[test]
    fn primary_only_mapping_depends_on_primary() {
        let deps = mapping_dependencies(&mapping(vec![Action::MapToVJoy { output: output() }]));

        assert_eq!(deps, vec![axis(0)]);
    }

    #[test]
    fn merge_mapping_depends_on_primary_and_secondary() {
        let deps = mapping_dependencies(&mapping(vec![Action::MergeAxis {
            second_input: axis(1),
            operation: MergeOp::Average,
        }]));

        assert_eq!(deps, vec![axis(0), axis(1)]);
    }

    #[test]
    fn nested_condition_dependencies_are_collected_once() {
        let deps = mapping_dependencies(&mapping(vec![Action::Conditional {
            condition: Condition::All {
                conditions: vec![
                    Condition::ButtonPressed { input: button(0) },
                    Condition::Not {
                        condition: Box::new(Condition::AxisInRange {
                            input: axis(1),
                            min: 0.0,
                            max: 1.0,
                        }),
                    },
                ],
            },
            if_true: vec![Action::Conditional {
                condition: Condition::ButtonReleased { input: button(0) },
                if_true: Vec::new(),
                if_false: Vec::new(),
            }],
            if_false: Vec::new(),
        }]));

        assert_eq!(deps, vec![axis(0), button(0), axis(1)]);
    }

    #[test]
    fn unbound_dependencies_are_ignored() {
        let deps = mapping_dependencies(&mapping(vec![
            Action::MergeAxis {
                second_input: InputAddress::Unbound,
                operation: MergeOp::Average,
            },
            Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: InputAddress::Unbound,
                },
                if_true: Vec::new(),
                if_false: Vec::new(),
            },
        ]));

        assert_eq!(deps, vec![axis(0)]);
    }
}
