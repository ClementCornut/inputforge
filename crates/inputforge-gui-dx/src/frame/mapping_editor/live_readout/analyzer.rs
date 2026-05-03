// Rust guideline compliant 2026-05-03

#![expect(
    dead_code,
    reason = "Task 5 emits only inputs and outputs; later tasks populate chains and predicates."
)]

use inputforge_core::action::Action;
use inputforge_core::state::AppState;
use inputforge_core::types::{AxisPolarity, InputAddress, KeyCombo, MergeOp, OutputAddress};

/// Maximum action nesting analyzed for live readout.
///
/// This is high enough for editor built pipelines while bounding future
/// recursive walks through nested conditionals.
pub(super) const MAX_NESTED_ACTION_DEPTH: usize = 32;

/// Complete live readout analysis result.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct LiveReadoutModel {
    /// Inputs shown at the head of the pipeline.
    pub pipeline_inputs: Vec<InputAddress>,
    /// Conditions evaluated while shaping output rows.
    pub predicates: Vec<PredicateDescriptor>,
    /// Output rows derived from terminal mapping actions.
    pub outputs: Vec<OutputDescriptor>,
}

/// Output row with its action chain.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct OutputDescriptor {
    /// Target reached by the analyzed pipeline.
    pub destination: OutputDestination,
    /// Merge and conditional steps that affect this output.
    pub chain: Vec<ChainStep>,
    /// Whether the current action path can produce this output.
    pub is_active: bool,
    /// Polarity used when rendering the final output value.
    pub polarity: AxisPolarity,
}

/// Destination kind for output rows.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum OutputDestination {
    /// vJoy axis, button, or hat output.
    VJoy(OutputAddress),
    /// Keyboard key combination output.
    Keyboard(KeyCombo),
}

/// Transformation step shown in an output chain.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ChainStep {
    /// Merge applied to the primary pipeline value.
    Merge {
        /// Operation selected by the merge stage.
        operation: MergeOp,
        /// Secondary input consumed by this merge.
        secondary_input: InputAddress,
        /// Secondary value before display domain conversion.
        encoded_value: f64,
        /// Output polarity after applying this merge.
        polarity_at_step: AxisPolarity,
    },
    /// Conditional branch taken or skipped during analysis.
    Conditional {
        /// User visible condition summary.
        condition_label: String,
        /// Result of evaluating the condition.
        evaluated: bool,
        /// Branch selected by the condition outcome.
        branch: Branch,
    },
}

/// Branch selected by a conditional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Branch {
    /// The true branch of a condition.
    IfTrue,
    /// The false branch of a condition.
    IfFalse,
}

/// Predicate row shown beside conditional output state.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PredicateDescriptor {
    /// Predicate behavior used for dedup and display.
    pub kind: PredicateKind,
    /// Inputs read while evaluating this predicate.
    pub inputs: Vec<InputAddress>,
    /// Current evaluated state.
    pub state: bool,
    /// User visible predicate summary.
    pub label: String,
}

/// Predicate variants captured by the analyzer.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum PredicateKind {
    /// Button must be currently pressed.
    ButtonPressed,
    /// Button must be currently released.
    ButtonReleased,
    /// Axis must fall inside the inclusive range.
    AxisInRange {
        /// Inclusive lower bound.
        min: f64,
        /// Inclusive upper bound.
        max: f64,
    },
    /// Hat must match one of these directions.
    HatDirection {
        /// Accepted hat directions.
        directions: Vec<inputforge_core::types::HatDirection>,
    },
}

pub(super) fn analyze(
    actions: &[Action],
    primary: &InputAddress,
    _state: &AppState,
) -> LiveReadoutModel {
    let mut model = LiveReadoutModel {
        pipeline_inputs: vec![primary.clone()],
        predicates: Vec::new(),
        outputs: Vec::new(),
    };
    walk(actions, &mut model, 0);
    model
}

fn walk(actions: &[Action], model: &mut LiveReadoutModel, depth: usize) {
    if depth > MAX_NESTED_ACTION_DEPTH {
        return;
    }

    for action in actions {
        match action {
            Action::MergeAxis { second_input, .. } => {
                model.pipeline_inputs.push(second_input.clone());
            }
            Action::MapToVJoy { output } => {
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::VJoy(output.clone()),
                    chain: Vec::new(),
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                });
            }
            Action::MapToKeyboard { key } => {
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::Keyboard(key.clone()),
                    chain: Vec::new(),
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                });
            }
            Action::Conditional {
                if_true, if_false, ..
            } => {
                walk(if_true, model, depth + 1);
                walk(if_false, model, depth + 1);
            }
            Action::ResponseCurve { .. }
            | Action::Deadzone { .. }
            | Action::Invert
            | Action::ChangeMode { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId, OutputId, VJoyAxis};

    #[test]
    fn model_default_is_empty() {
        let model = LiveReadoutModel::default();

        assert!(model.pipeline_inputs.is_empty());
        assert!(model.predicates.is_empty());
        assert!(model.outputs.is_empty());
    }

    #[test]
    fn output_descriptor_carries_destination_chain_polarity() {
        let secondary_input = InputAddress::Bound {
            device: DeviceId("stick".into()),
            input: InputId::Axis { index: 1 },
        };
        let output = OutputDescriptor {
            destination: OutputDestination::VJoy(OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            }),
            chain: vec![ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input: secondary_input.clone(),
                encoded_value: 0.25,
                polarity_at_step: AxisPolarity::Bipolar,
            }],
            is_active: true,
            polarity: AxisPolarity::Bipolar,
        };

        assert_eq!(output.chain.len(), 1);
        assert_eq!(output.polarity, AxisPolarity::Bipolar);
        assert!(output.is_active);
        assert_eq!(
            output.chain[0],
            ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input,
                encoded_value: 0.25,
                polarity_at_step: AxisPolarity::Bipolar,
            }
        );
    }
}

#[cfg(test)]
mod walker_tests {
    use super::*;
    use inputforge_core::action::Condition;
    use inputforge_core::types::{DeviceId, InputId, KeyModifier, OutputId, VJoyAxis};

    fn input(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("stick".to_owned()),
            input: InputId::Axis { index },
        }
    }

    fn button(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("stick".to_owned()),
            input: InputId::Button { index },
        }
    }

    fn vjoy_axis(axis: VJoyAxis) -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: axis },
        }
    }

    fn keyboard_combo(key: &str) -> KeyCombo {
        KeyCombo {
            key: key.to_owned(),
            modifiers: vec![KeyModifier::Ctrl, KeyModifier::Shift],
        }
    }

    fn condition() -> Condition {
        Condition::ButtonPressed { input: button(0) }
    }

    fn nested_conditional(levels: usize, terminal: Action) -> Action {
        let mut action = terminal;
        for _ in 0..levels {
            action = Action::Conditional {
                condition: condition(),
                if_true: vec![action],
                if_false: Vec::new(),
            };
        }
        action
    }

    fn analyze_actions(actions: &[Action], primary: &InputAddress) -> LiveReadoutModel {
        analyze(actions, primary, &AppState::new())
    }

    #[test]
    fn empty_actions_yields_only_primary_input() {
        let primary = input(0);

        let model = analyze_actions(&[], &primary);

        assert_eq!(model.pipeline_inputs, vec![primary]);
        assert!(model.predicates.is_empty());
        assert!(model.outputs.is_empty());
    }

    #[test]
    fn stacked_merges_emit_primary_plus_secondaries_in_order() {
        let primary = input(0);
        let second = input(1);
        let third = input(2);
        let actions = vec![
            Action::MergeAxis {
                second_input: second.clone(),
                operation: MergeOp::Average,
            },
            Action::MergeAxis {
                second_input: third.clone(),
                operation: MergeOp::Maximum,
            },
        ];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.pipeline_inputs, vec![primary, second, third]);
        assert!(model.outputs.is_empty());
    }

    #[test]
    fn sibling_outputs_yield_one_descriptor_each() {
        let primary = input(0);
        let first = vjoy_axis(VJoyAxis::X);
        let second = vjoy_axis(VJoyAxis::Y);
        let actions = vec![
            Action::MapToVJoy {
                output: first.clone(),
            },
            Action::MapToVJoy {
                output: second.clone(),
            },
        ];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.outputs.len(), 2);
        assert_eq!(
            model.outputs[0],
            OutputDescriptor {
                destination: OutputDestination::VJoy(first),
                chain: Vec::new(),
                is_active: true,
                polarity: AxisPolarity::Bipolar,
            }
        );
        assert_eq!(
            model.outputs[1],
            OutputDescriptor {
                destination: OutputDestination::VJoy(second),
                chain: Vec::new(),
                is_active: true,
                polarity: AxisPolarity::Bipolar,
            }
        );
    }

    #[test]
    fn keyboard_output_yields_keyboard_destination() {
        let primary = input(0);
        let key = keyboard_combo("F1");
        let actions = vec![Action::MapToKeyboard { key: key.clone() }];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(
            model.outputs,
            vec![OutputDescriptor {
                destination: OutputDestination::Keyboard(key),
                chain: Vec::new(),
                is_active: true,
                polarity: AxisPolarity::Bipolar,
            }]
        );
    }

    #[test]
    fn conditional_outputs_in_both_branches_are_emitted() {
        let primary = input(0);
        let true_output = vjoy_axis(VJoyAxis::X);
        let false_output = vjoy_axis(VJoyAxis::Y);
        let actions = vec![Action::Conditional {
            condition: condition(),
            if_true: vec![Action::MapToVJoy {
                output: true_output.clone(),
            }],
            if_false: vec![Action::MapToVJoy {
                output: false_output.clone(),
            }],
        }];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(
            model.outputs,
            vec![
                OutputDescriptor {
                    destination: OutputDestination::VJoy(true_output),
                    chain: Vec::new(),
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                },
                OutputDescriptor {
                    destination: OutputDestination::VJoy(false_output),
                    chain: Vec::new(),
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                },
            ]
        );
    }

    #[test]
    fn walker_caps_at_max_nested_depth() {
        let primary = input(0);
        let actions = vec![nested_conditional(
            MAX_NESTED_ACTION_DEPTH + 1,
            Action::MapToVJoy {
                output: vjoy_axis(VJoyAxis::X),
            },
        )];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.pipeline_inputs, vec![primary]);
        assert!(model.outputs.is_empty());
    }
}
