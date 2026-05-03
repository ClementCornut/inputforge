// Rust guideline compliant 2026-05-03

#![expect(
    dead_code,
    reason = "Task 5 emits only inputs and outputs; later tasks populate chains and predicates."
)]

use inputforge_core::action::Action;
use inputforge_core::pipeline::{
    BranchStep, InputCache, evaluate_actions_through_path, evaluate_condition,
};
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
    state: &AppState,
) -> LiveReadoutModel {
    let mut model = LiveReadoutModel {
        pipeline_inputs: vec![primary.clone()],
        predicates: Vec::new(),
        outputs: Vec::new(),
    };
    let context = AnalysisContext {
        top_level: actions,
        primary,
        state,
    };
    let mut chain_stack = Vec::new();
    let mut branch_path = Vec::new();
    walk(
        &context,
        actions,
        &mut model,
        &mut chain_stack,
        &mut branch_path,
        0,
    );
    model
}

struct AnalysisContext<'a> {
    top_level: &'a [Action],
    primary: &'a InputAddress,
    state: &'a AppState,
}

impl AnalysisContext<'_> {
    fn compute_merge_step_data(
        &self,
        branch_path: &[BranchStep],
        chain_stack: &[ChainStep],
        local_idx: usize,
        second_input: &InputAddress,
        operation: MergeOp,
    ) -> (f64, AxisPolarity) {
        let actions = self.flatten_actions_to_merge(branch_path, local_idx);
        let input_value =
            evaluate_actions_through_path(&actions, self.state, self.primary, &[], actions.len());
        let encoded_value = super::value_helpers::axis_f64(&input_value);
        let primary_polarity = chain_stack
            .iter()
            .rev()
            .find_map(|step| match step {
                ChainStep::Merge {
                    polarity_at_step, ..
                } => Some(*polarity_at_step),
                ChainStep::Conditional { .. } => None,
            })
            .unwrap_or_else(|| self.state.input_cache.get_axis(self.primary).1);
        let secondary_polarity = self.state.input_cache.get_axis(second_input).1;
        let polarity_at_step = super::value_helpers::merge_output_polarity(
            operation,
            primary_polarity,
            secondary_polarity,
        );

        (encoded_value, polarity_at_step)
    }

    fn flatten_actions_to_merge(
        &self,
        branch_path: &[BranchStep],
        local_idx: usize,
    ) -> Vec<Action> {
        let mut flattened = Vec::new();
        let mut current = self.top_level;

        for step in branch_path {
            let (index, wants_true) = match *step {
                BranchStep::IfTrue(index) => (index, true),
                BranchStep::IfFalse(index) => (index, false),
            };
            append_non_conditional_actions(&current[..index], &mut flattened);
            match &current[index] {
                Action::Conditional {
                    if_true, if_false, ..
                } => {
                    current = if wants_true { if_true } else { if_false };
                }
                other => {
                    panic!("branch path target at index {index} must be Conditional, got {other:?}")
                }
            }
        }

        append_non_conditional_actions(&current[..=local_idx], &mut flattened);
        flattened
    }
}

fn append_non_conditional_actions(actions: &[Action], out: &mut Vec<Action>) {
    out.extend(
        actions
            .iter()
            .filter(|action| !matches!(action, Action::Conditional { .. }))
            .cloned(),
    );
}

fn compute_is_active(chain: &[ChainStep]) -> bool {
    chain.iter().all(|step| match step {
        ChainStep::Merge { .. } => true,
        ChainStep::Conditional {
            evaluated, branch, ..
        } => *evaluated == matches!(branch, Branch::IfTrue),
    })
}

fn terminal_polarity(
    chain: &[ChainStep],
    primary: &InputAddress,
    state: &AppState,
) -> AxisPolarity {
    chain
        .iter()
        .rev()
        .find_map(|step| match step {
            ChainStep::Merge {
                polarity_at_step, ..
            } => Some(*polarity_at_step),
            ChainStep::Conditional { .. } => None,
        })
        .unwrap_or_else(|| state.input_cache.get_axis(primary).1)
}

fn walk(
    context: &AnalysisContext<'_>,
    actions: &[Action],
    model: &mut LiveReadoutModel,
    chain_stack: &mut Vec<ChainStep>,
    branch_path: &mut Vec<BranchStep>,
    depth: usize,
) {
    let stack_baseline = chain_stack.len();
    if depth > MAX_NESTED_ACTION_DEPTH {
        chain_stack.truncate(stack_baseline);
        return;
    }

    for (i, action) in actions.iter().enumerate() {
        match action {
            Action::MergeAxis {
                second_input,
                operation,
            } => {
                model.pipeline_inputs.push(second_input.clone());
                let (encoded_value, polarity_at_step) = context.compute_merge_step_data(
                    branch_path,
                    chain_stack,
                    i,
                    second_input,
                    *operation,
                );
                chain_stack.push(ChainStep::Merge {
                    operation: *operation,
                    secondary_input: second_input.clone(),
                    encoded_value,
                    polarity_at_step,
                });
            }
            Action::MapToVJoy { output } => {
                let is_active = compute_is_active(chain_stack);
                let polarity = terminal_polarity(chain_stack, context.primary, context.state);
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::VJoy(output.clone()),
                    chain: chain_stack.clone(),
                    is_active,
                    polarity,
                });
            }
            Action::MapToKeyboard { key } => {
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::Keyboard(key.clone()),
                    chain: chain_stack.clone(),
                    is_active: compute_is_active(chain_stack),
                    polarity: AxisPolarity::Bipolar,
                });
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                let condition_label = format!("{condition:?}");
                let evaluated = evaluate_condition(condition, &context.state.input_cache);

                branch_path.push(BranchStep::IfTrue(i));
                chain_stack.push(ChainStep::Conditional {
                    condition_label: condition_label.clone(),
                    evaluated,
                    branch: Branch::IfTrue,
                });
                walk(context, if_true, model, chain_stack, branch_path, depth + 1);
                chain_stack.pop();
                branch_path.pop();

                branch_path.push(BranchStep::IfFalse(i));
                chain_stack.push(ChainStep::Conditional {
                    condition_label,
                    evaluated,
                    branch: Branch::IfFalse,
                });
                walk(
                    context,
                    if_false,
                    model,
                    chain_stack,
                    branch_path,
                    depth + 1,
                );
                chain_stack.pop();
                branch_path.pop();
            }
            Action::ResponseCurve { .. }
            | Action::Deadzone { .. }
            | Action::Invert
            | Action::ChangeMode { .. } => {}
        }
    }

    chain_stack.truncate(stack_baseline);
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
    use inputforge_core::types::{
        AxisValue, DeviceId, InputId, InputValue, KeyModifier, OutputId, VJoyAxis,
    };

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

    fn analyze_actions_with_state(
        actions: &[Action],
        primary: &InputAddress,
        state: &AppState,
    ) -> LiveReadoutModel {
        analyze(actions, primary, state)
    }

    fn set_axis(state: &mut AppState, input: &InputAddress, value: f64, polarity: AxisPolarity) {
        state.input_cache.update(
            input,
            &InputValue::Axis {
                value: AxisValue::new(value),
                polarity,
            },
        );
    }

    fn set_button(state: &mut AppState, input: &InputAddress, pressed: bool) {
        state
            .input_cache
            .update(input, &InputValue::Button { pressed });
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
    fn merge_then_output_records_one_chain_step() {
        let primary = input(0);
        let second = input(1);
        let output = vjoy_axis(VJoyAxis::X);
        let mut state = AppState::new();
        set_axis(&mut state, &primary, 0.5, AxisPolarity::Bipolar);
        set_axis(&mut state, &second, 0.25, AxisPolarity::Bipolar);
        let actions = vec![
            Action::MergeAxis {
                second_input: second.clone(),
                operation: MergeOp::Average,
            },
            Action::MapToVJoy {
                output: output.clone(),
            },
        ];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(
            model.outputs,
            vec![OutputDescriptor {
                destination: OutputDestination::VJoy(output),
                chain: vec![ChainStep::Merge {
                    operation: MergeOp::Average,
                    secondary_input: second,
                    encoded_value: 0.375,
                    polarity_at_step: AxisPolarity::Bipolar,
                }],
                is_active: true,
                polarity: AxisPolarity::Bipolar,
            }]
        );
    }

    #[test]
    fn stacked_merges_record_two_chain_steps_for_each_output() {
        let primary = input(0);
        let second = input(1);
        let third = input(2);
        let first_output = vjoy_axis(VJoyAxis::X);
        let second_output = vjoy_axis(VJoyAxis::Y);
        let mut state = AppState::new();
        set_axis(&mut state, &primary, 0.2, AxisPolarity::Bipolar);
        set_axis(&mut state, &second, 0.6, AxisPolarity::Bipolar);
        set_axis(&mut state, &third, -0.4, AxisPolarity::Bipolar);
        let actions = vec![
            Action::MergeAxis {
                second_input: second.clone(),
                operation: MergeOp::Average,
            },
            Action::MergeAxis {
                second_input: third.clone(),
                operation: MergeOp::Average,
            },
            Action::MapToVJoy {
                output: first_output,
            },
            Action::MapToVJoy {
                output: second_output,
            },
        ];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        let expected_chain = vec![
            ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input: second,
                encoded_value: 0.4,
                polarity_at_step: AxisPolarity::Bipolar,
            },
            ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input: third,
                encoded_value: 0.0,
                polarity_at_step: AxisPolarity::Bipolar,
            },
        ];
        assert_eq!(model.outputs.len(), 2);
        assert_eq!(model.outputs[0].chain, expected_chain);
        assert_eq!(model.outputs[1].chain, model.outputs[0].chain);
    }

    #[test]
    fn sibling_outputs_share_the_pre_split_merges_only() {
        let primary = input(0);
        let pre_split = input(1);
        let nested = input(2);
        let true_output = vjoy_axis(VJoyAxis::X);
        let false_output = vjoy_axis(VJoyAxis::Y);
        let after_output = vjoy_axis(VJoyAxis::Z);
        let mut state = AppState::new();
        set_axis(&mut state, &primary, 0.2, AxisPolarity::Bipolar);
        set_axis(&mut state, &pre_split, 0.6, AxisPolarity::Bipolar);
        set_axis(&mut state, &nested, -0.4, AxisPolarity::Bipolar);
        let actions = vec![
            Action::MergeAxis {
                second_input: pre_split.clone(),
                operation: MergeOp::Average,
            },
            Action::Conditional {
                condition: condition(),
                if_true: vec![
                    Action::MergeAxis {
                        second_input: nested.clone(),
                        operation: MergeOp::Average,
                    },
                    Action::MapToVJoy {
                        output: true_output,
                    },
                ],
                if_false: vec![Action::MapToVJoy {
                    output: false_output,
                }],
            },
            Action::MapToVJoy {
                output: after_output,
            },
        ];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        let pre_split_step = ChainStep::Merge {
            operation: MergeOp::Average,
            secondary_input: pre_split,
            encoded_value: 0.4,
            polarity_at_step: AxisPolarity::Bipolar,
        };
        let condition_step = ChainStep::Conditional {
            condition_label: format!("{:?}", condition()),
            evaluated: false,
            branch: Branch::IfTrue,
        };
        assert_eq!(model.outputs.len(), 3);
        assert_eq!(
            model.outputs[0].chain,
            vec![
                pre_split_step.clone(),
                condition_step,
                ChainStep::Merge {
                    operation: MergeOp::Average,
                    secondary_input: nested,
                    encoded_value: 0.0,
                    polarity_at_step: AxisPolarity::Bipolar,
                },
            ]
        );
        assert_eq!(
            model.outputs[1].chain,
            vec![
                pre_split_step.clone(),
                ChainStep::Conditional {
                    condition_label: format!("{:?}", condition()),
                    evaluated: false,
                    branch: Branch::IfFalse,
                },
            ]
        );
        assert_eq!(model.outputs[2].chain, vec![pre_split_step]);
    }

    #[test]
    fn top_level_merge_after_conditional_ignores_nested_branch_merge_value() {
        let primary = input(0);
        let nested = input(1);
        let later = input(2);
        let output = vjoy_axis(VJoyAxis::X);
        let mut state = AppState::new();
        set_axis(&mut state, &primary, 0.0, AxisPolarity::Bipolar);
        set_axis(&mut state, &nested, 1.0, AxisPolarity::Bipolar);
        set_axis(&mut state, &later, -1.0, AxisPolarity::Bipolar);
        set_button(&mut state, &button(0), true);
        let actions = vec![
            Action::Conditional {
                condition: condition(),
                if_true: vec![Action::MergeAxis {
                    second_input: nested,
                    operation: MergeOp::Average,
                }],
                if_false: Vec::new(),
            },
            Action::MergeAxis {
                second_input: later.clone(),
                operation: MergeOp::Average,
            },
            Action::MapToVJoy { output },
        ];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(
            model.outputs[0].chain,
            vec![ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input: later,
                encoded_value: -0.5,
                polarity_at_step: AxisPolarity::Bipolar,
            }]
        );
    }

    #[test]
    fn merge_step_carries_polarity_at_step_not_terminal() {
        let primary = input(0);
        let second = input(1);
        let third = input(2);
        let output = vjoy_axis(VJoyAxis::X);
        let mut state = AppState::new();
        set_axis(&mut state, &primary, -0.2, AxisPolarity::Unipolar);
        set_axis(&mut state, &second, 0.2, AxisPolarity::Unipolar);
        set_axis(&mut state, &third, 0.4, AxisPolarity::Bipolar);
        let actions = vec![
            Action::MergeAxis {
                second_input: second.clone(),
                operation: MergeOp::Average,
            },
            Action::MergeAxis {
                second_input: third.clone(),
                operation: MergeOp::Average,
            },
            Action::MapToVJoy { output },
        ];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(
            model.outputs[0].chain,
            vec![
                ChainStep::Merge {
                    operation: MergeOp::Average,
                    secondary_input: second,
                    encoded_value: 0.0,
                    polarity_at_step: AxisPolarity::Unipolar,
                },
                ChainStep::Merge {
                    operation: MergeOp::Average,
                    secondary_input: third,
                    encoded_value: 0.2,
                    polarity_at_step: AxisPolarity::Bipolar,
                },
            ]
        );
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
        let test_condition = condition();
        let condition_label = format!("{test_condition:?}");
        let actions = vec![Action::Conditional {
            condition: test_condition,
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
                    chain: vec![ChainStep::Conditional {
                        condition_label: condition_label.clone(),
                        evaluated: false,
                        branch: Branch::IfTrue,
                    }],
                    is_active: false,
                    polarity: AxisPolarity::Bipolar,
                },
                OutputDescriptor {
                    destination: OutputDestination::VJoy(false_output),
                    chain: vec![ChainStep::Conditional {
                        condition_label,
                        evaluated: false,
                        branch: Branch::IfFalse,
                    }],
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                },
            ]
        );
    }

    #[test]
    fn conditional_branches_record_distinct_chain_steps() {
        let primary = input(0);
        let true_output = vjoy_axis(VJoyAxis::X);
        let false_output = vjoy_axis(VJoyAxis::Y);
        let test_condition = condition();
        let condition_label = format!("{test_condition:?}");
        let mut state = AppState::new();
        set_button(&mut state, &button(0), true);
        let actions = vec![Action::Conditional {
            condition: test_condition,
            if_true: vec![Action::MapToVJoy {
                output: true_output.clone(),
            }],
            if_false: vec![Action::MapToVJoy {
                output: false_output.clone(),
            }],
        }];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(
            model.outputs,
            vec![
                OutputDescriptor {
                    destination: OutputDestination::VJoy(true_output),
                    chain: vec![ChainStep::Conditional {
                        condition_label: condition_label.clone(),
                        evaluated: true,
                        branch: Branch::IfTrue,
                    }],
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                },
                OutputDescriptor {
                    destination: OutputDestination::VJoy(false_output),
                    chain: vec![ChainStep::Conditional {
                        condition_label,
                        evaluated: true,
                        branch: Branch::IfFalse,
                    }],
                    is_active: false,
                    polarity: AxisPolarity::Bipolar,
                },
            ]
        );
    }

    #[test]
    fn is_active_true_when_predicate_matches_branch() {
        let primary = input(0);
        let true_output = vjoy_axis(VJoyAxis::X);
        let false_output = vjoy_axis(VJoyAxis::Y);
        let mut state = AppState::new();
        set_button(&mut state, &button(0), true);
        let actions = vec![Action::Conditional {
            condition: condition(),
            if_true: vec![Action::MapToVJoy {
                output: true_output,
            }],
            if_false: vec![Action::MapToVJoy {
                output: false_output,
            }],
        }];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(model.outputs.len(), 2);
        assert!(model.outputs[0].is_active);
        assert!(!model.outputs[1].is_active);
    }

    #[test]
    fn is_active_nested_path_and_evaluation() {
        let primary = input(0);
        let active_output = vjoy_axis(VJoyAxis::X);
        let inactive_output = vjoy_axis(VJoyAxis::Y);
        let inner_condition = Condition::ButtonPressed { input: button(1) };
        let mut state = AppState::new();
        set_button(&mut state, &button(0), true);
        set_button(&mut state, &button(1), false);
        let actions = vec![Action::Conditional {
            condition: condition(),
            if_true: vec![Action::Conditional {
                condition: inner_condition,
                if_true: vec![Action::MapToVJoy {
                    output: inactive_output,
                }],
                if_false: vec![Action::MapToVJoy {
                    output: active_output,
                }],
            }],
            if_false: Vec::new(),
        }];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(model.outputs.len(), 2);
        assert!(!model.outputs[0].is_active);
        assert!(model.outputs[1].is_active);
    }

    #[test]
    fn polarity_no_merges_inherits_primary() {
        let primary = input(0);
        let output = vjoy_axis(VJoyAxis::X);
        let mut state = AppState::new();
        set_axis(&mut state, &primary, 0.25, AxisPolarity::Unipolar);
        let actions = vec![Action::MapToVJoy {
            output: output.clone(),
        }];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(
            model.outputs,
            vec![OutputDescriptor {
                destination: OutputDestination::VJoy(output),
                chain: Vec::new(),
                is_active: true,
                polarity: AxisPolarity::Unipolar,
            }]
        );
    }

    #[test]
    fn nested_conditional_records_two_chain_steps() {
        let primary = input(0);
        let output = vjoy_axis(VJoyAxis::X);
        let outer_condition = condition();
        let inner_condition = Condition::ButtonReleased { input: button(1) };
        let outer_label = format!("{outer_condition:?}");
        let inner_label = format!("{inner_condition:?}");
        let mut state = AppState::new();
        set_button(&mut state, &button(0), true);
        set_button(&mut state, &button(1), false);
        let actions = vec![Action::Conditional {
            condition: outer_condition,
            if_true: vec![Action::Conditional {
                condition: inner_condition,
                if_true: vec![Action::MapToVJoy {
                    output: output.clone(),
                }],
                if_false: Vec::new(),
            }],
            if_false: Vec::new(),
        }];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(
            model.outputs,
            vec![OutputDescriptor {
                destination: OutputDestination::VJoy(output),
                chain: vec![
                    ChainStep::Conditional {
                        condition_label: outer_label,
                        evaluated: true,
                        branch: Branch::IfTrue,
                    },
                    ChainStep::Conditional {
                        condition_label: inner_label,
                        evaluated: true,
                        branch: Branch::IfTrue,
                    },
                ],
                is_active: true,
                polarity: AxisPolarity::Bipolar,
            }]
        );
    }

    #[test]
    fn conditional_evaluated_uses_input_cache() {
        let primary = input(0);
        let watched_axis = input(1);
        let output = vjoy_axis(VJoyAxis::X);
        let test_condition = Condition::AxisInRange {
            input: watched_axis.clone(),
            min: 0.25,
            max: 0.75,
        };
        let condition_label = format!("{test_condition:?}");
        let mut state = AppState::new();
        set_axis(&mut state, &watched_axis, 0.5, AxisPolarity::Bipolar);
        let actions = vec![Action::Conditional {
            condition: test_condition,
            if_true: vec![Action::MapToVJoy {
                output: output.clone(),
            }],
            if_false: Vec::new(),
        }];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(
            model.outputs,
            vec![OutputDescriptor {
                destination: OutputDestination::VJoy(output),
                chain: vec![ChainStep::Conditional {
                    condition_label,
                    evaluated: true,
                    branch: Branch::IfTrue,
                }],
                is_active: true,
                polarity: AxisPolarity::Bipolar,
            }]
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
