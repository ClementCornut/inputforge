//! Characterization tests moved with app-owned analysis.
use super::{analyze::analyze, walk::MAX_NESTED_ACTION_DEPTH};
use inputforge_core::{
    action::{Action, ActionBranch, Condition, MouseTarget, OutputBehavior},
    pipeline::{OutputDestination as ActivityDestination, OutputOwner},
    state::AppState,
    types::{AxisPolarity, HatDirection, InputAddress, KeyCombo, MergeOp, OutputAddress},
};
use inputforge_gui_dx::{
    readouts::{
        Branch, ChainStep, LiveReadoutModel, OutputDescriptor, OutputDestination,
        PredicateDescriptor, PredicateKind,
    },
    snapshots::ConfigSnapshot,
};
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId, OutputId, VJoyAxis};

    #[test]
    fn model_default_is_empty() {
        let model = LiveReadoutModel::default();

        assert_eq!(model.pipeline_inputs.len(), 0);
        assert_eq!(model.predicates.len(), 0);
        assert_eq!(model.outputs.len(), 0);
    }

    #[test]
    fn output_descriptor_carries_destination_chain_polarity() {
        let secondary_input = InputAddress::Bound {
            device: DeviceId("stick".into()),
            input: InputId::Axis { index: 1 },
        };
        let output = OutputDescriptor {
            owner: OutputOwner {
                profile: "memory-profile".into(),
                mode: "Default".into(),
                input: InputAddress::Unbound,
                action_path: vec![],
                destination: ActivityDestination::VJoy(OutputAddress {
                    device: 1,
                    output: OutputId::Axis { id: VJoyAxis::X },
                }),
                behavior: OutputBehavior::Hold,
            },
            destination: OutputDestination::VJoy(OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            }),
            chain: vec![ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input: secondary_input.clone(),
                display_value: 0.25,
                polarity_at_step: AxisPolarity::Bipolar,
            }],
            is_active: true,
            polarity: AxisPolarity::Bipolar,
            activity: None,
        };

        assert_eq!(output.chain.len(), 1);
        assert_eq!(output.polarity, AxisPolarity::Bipolar);
        assert!(output.is_active);
        assert_eq!(
            output.chain[0],
            ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input,
                display_value: 0.25,
                polarity_at_step: AxisPolarity::Bipolar,
            }
        );
    }
}

#[cfg(test)]
mod walker_tests {
    use super::*;
    use inputforge_core::types::{
        AxisValue, DeviceId, InputId, InputValue, KeyModifier, OutputId, PhysicalKey, VJoyAxis,
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

    fn keyboard_combo(key: PhysicalKey) -> KeyCombo {
        KeyCombo {
            key,
            modifiers: vec![KeyModifier::CONTROL_LEFT, KeyModifier::SHIFT_LEFT],
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
        analyze(
            actions,
            primary,
            &AppState::new(),
            &ConfigSnapshot::default(),
        )
    }

    fn analyze_actions_with_state(
        actions: &[Action],
        primary: &InputAddress,
        state: &AppState,
    ) -> LiveReadoutModel {
        analyze(actions, primary, state, &ConfigSnapshot::default())
    }

    fn condition_label(condition: &Condition) -> String {
        inputforge_gui_dx::readouts::labels::format_condition_label(
            condition,
            &ConfigSnapshot::default(),
        )
    }

    fn conditional_action(condition: Condition) -> Action {
        Action::Conditional {
            condition,
            if_true: Vec::new(),
            if_false: Vec::new(),
        }
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
        assert_eq!(model.predicates.len(), 0);
        assert_eq!(model.outputs.len(), 0);
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
        assert_eq!(model.outputs.len(), 0);
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
                owner: model.outputs[0].owner.clone(),
                destination: OutputDestination::VJoy(output),
                chain: vec![ChainStep::Merge {
                    operation: MergeOp::Average,
                    secondary_input: second,
                    display_value: 0.375,
                    polarity_at_step: AxisPolarity::Bipolar,
                }],
                is_active: true,
                polarity: AxisPolarity::Bipolar,
                activity: None,
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
                display_value: 0.4,
                polarity_at_step: AxisPolarity::Bipolar,
            },
            ChainStep::Merge {
                operation: MergeOp::Average,
                secondary_input: third,
                display_value: 0.0,
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
            display_value: 0.4,
            polarity_at_step: AxisPolarity::Bipolar,
        };
        let condition_step = ChainStep::Conditional {
            condition_label: condition_label(&condition()),
            active: false,
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
                    display_value: 0.0,
                    polarity_at_step: AxisPolarity::Bipolar,
                },
            ]
        );
        assert_eq!(
            model.outputs[1].chain,
            vec![
                pre_split_step.clone(),
                ChainStep::Conditional {
                    condition_label: condition_label(&condition()),
                    active: true,
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
                display_value: -0.5,
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
                    display_value: 0.5,
                    polarity_at_step: AxisPolarity::Unipolar,
                },
                ChainStep::Merge {
                    operation: MergeOp::Average,
                    secondary_input: third,
                    display_value: 0.2,
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
                owner: model.outputs[0].owner.clone(),
                destination: OutputDestination::VJoy(first),
                chain: Vec::new(),
                is_active: true,
                polarity: AxisPolarity::Bipolar,
                activity: None,
            }
        );
        assert_eq!(
            model.outputs[1],
            OutputDescriptor {
                owner: model.outputs[1].owner.clone(),
                destination: OutputDestination::VJoy(second),
                chain: Vec::new(),
                is_active: true,
                polarity: AxisPolarity::Bipolar,
                activity: None,
            }
        );
    }

    #[test]
    fn keyboard_output_yields_keyboard_destination() {
        let primary = input(0);
        let key = keyboard_combo(PhysicalKey::F1);
        let actions = vec![Action::MapToKeyboard {
            key: key.clone(),
            behavior: OutputBehavior::Hold,
        }];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(
            model.outputs,
            vec![OutputDescriptor {
                owner: model.outputs[0].owner.clone(),
                destination: OutputDestination::Keyboard {
                    key,
                    behavior: OutputBehavior::Hold,
                    pressed: false,
                },
                chain: Vec::new(),
                is_active: true,
                polarity: AxisPolarity::Bipolar,
                activity: None,
            }]
        );
    }

    #[test]
    fn live_readout_exposes_mouse_destination() {
        let actions = vec![Action::MapToMouse {
            target: MouseTarget::RightButton,
            behavior: OutputBehavior::Hold,
        }];
        let model = analyze_actions(&actions, &input(0));

        assert!(matches!(
            model.outputs[0].destination,
            OutputDestination::Mouse {
                target: MouseTarget::RightButton,
                behavior: OutputBehavior::Hold,
                active: false,
            }
        ));
    }

    #[test]
    fn conditional_outputs_in_both_branches_are_emitted() {
        let primary = input(0);
        let true_output = vjoy_axis(VJoyAxis::X);
        let false_output = vjoy_axis(VJoyAxis::Y);
        let test_condition = condition();
        let condition_label = condition_label(&test_condition);
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
                    owner: model.outputs[0].owner.clone(),
                    destination: OutputDestination::VJoy(true_output),
                    chain: vec![ChainStep::Conditional {
                        condition_label: condition_label.clone(),
                        active: false,
                        branch: Branch::IfTrue,
                    }],
                    is_active: false,
                    polarity: AxisPolarity::Bipolar,
                    activity: None,
                },
                OutputDescriptor {
                    owner: model.outputs[1].owner.clone(),
                    destination: OutputDestination::VJoy(false_output),
                    chain: vec![ChainStep::Conditional {
                        condition_label,
                        active: true,
                        branch: Branch::IfFalse,
                    }],
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                    activity: None,
                },
            ]
        );
    }

    #[test]
    fn gesture_branches_emit_output_descriptors() {
        let primary = input(0);
        let secondary = input(1);
        let single_output = vjoy_axis(VJoyAxis::X);
        let double_output = vjoy_axis(VJoyAxis::Y);
        let mut state = AppState::new();
        set_axis(&mut state, &primary, 0.25, AxisPolarity::Bipolar);
        set_axis(&mut state, &secondary, 0.75, AxisPolarity::Bipolar);
        let actions = vec![Action::TapGesture {
            threshold_ms: 300,
            fire_single_immediately: false,
            single_tap: vec![Action::MapToVJoy {
                output: single_output.clone(),
            }],
            double_tap: vec![
                Action::MergeAxis {
                    second_input: secondary.clone(),
                    operation: MergeOp::Average,
                },
                Action::MapToVJoy {
                    output: double_output.clone(),
                },
            ],
        }];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(model.outputs.len(), 2);
        assert_eq!(
            model.outputs[0],
            OutputDescriptor {
                owner: model.outputs[0].owner.clone(),
                destination: OutputDestination::VJoy(single_output),
                chain: vec![ChainStep::Conditional {
                    condition_label: "Single tap".to_owned(),
                    active: false,
                    branch: Branch::Gesture(ActionBranch::TapSingle),
                }],
                is_active: false,
                polarity: AxisPolarity::Bipolar,
                activity: None,
            }
        );
        assert_eq!(
            model.outputs[1],
            OutputDescriptor {
                owner: model.outputs[1].owner.clone(),
                destination: OutputDestination::VJoy(double_output),
                chain: vec![
                    ChainStep::Conditional {
                        condition_label: "Double tap".to_owned(),
                        active: false,
                        branch: Branch::Gesture(ActionBranch::TapDouble),
                    },
                    ChainStep::Merge {
                        operation: MergeOp::Average,
                        secondary_input: secondary.clone(),
                        display_value: 0.5,
                        polarity_at_step: AxisPolarity::Bipolar,
                    },
                ],
                is_active: false,
                polarity: AxisPolarity::Bipolar,
                activity: None,
            }
        );
        assert_eq!(model.pipeline_inputs, vec![primary, secondary]);
        assert_eq!(model.predicates.len(), 0);
    }

    #[test]
    fn conditional_emits_predicate_descriptor() {
        let primary = input(0);
        let watched = button(1);
        let mut state = AppState::new();
        set_button(&mut state, &watched, true);
        let actions = vec![conditional_action(Condition::ButtonPressed {
            input: watched.clone(),
        })];

        let model = analyze_actions_with_state(&actions, &primary, &state);

        assert_eq!(
            model.predicates,
            vec![PredicateDescriptor {
                kind: PredicateKind::ButtonPressed,
                inputs: vec![watched.clone()],
                state: true,
                label: inputforge_gui_dx::readouts::labels::format_predicate_chip_label(
                    &watched,
                    &ConfigSnapshot::default()
                ),
            }]
        );
    }

    #[test]
    fn composite_all_flattens_to_one_chip_per_leaf() {
        let primary = input(0);
        let first = button(1);
        let second = input(2);
        let actions = vec![conditional_action(Condition::All {
            conditions: vec![
                Condition::ButtonPressed {
                    input: first.clone(),
                },
                Condition::AxisInRange {
                    input: second.clone(),
                    min: 0.25,
                    max: 0.75,
                },
            ],
        })];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 2);
        assert_eq!(model.predicates[0].kind, PredicateKind::ButtonPressed);
        assert_eq!(model.predicates[0].inputs, vec![first]);
        assert_eq!(
            model.predicates[1].kind,
            PredicateKind::AxisInRange {
                min: 0.25,
                max: 0.75,
            }
        );
        assert_eq!(model.predicates[1].inputs, vec![second]);
    }

    #[test]
    fn composite_any_flattens_to_one_chip_per_leaf() {
        let primary = input(0);
        let first = button(1);
        let second = input(2);
        let actions = vec![conditional_action(Condition::Any {
            conditions: vec![
                Condition::ButtonReleased {
                    input: first.clone(),
                },
                Condition::AxisInRange {
                    input: second.clone(),
                    min: -0.25,
                    max: 0.25,
                },
            ],
        })];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 2);
        assert_eq!(model.predicates[0].kind, PredicateKind::ButtonReleased);
        assert_eq!(model.predicates[0].inputs, vec![first]);
        assert_eq!(
            model.predicates[1].kind,
            PredicateKind::AxisInRange {
                min: -0.25,
                max: 0.25,
            }
        );
        assert_eq!(model.predicates[1].inputs, vec![second]);
    }

    #[test]
    fn composite_not_flattens_to_inner_leaves() {
        let primary = input(0);
        let first = button(1);
        let second = button(2);
        let actions = vec![conditional_action(Condition::Not {
            condition: Box::new(Condition::All {
                conditions: vec![
                    Condition::ButtonPressed {
                        input: first.clone(),
                    },
                    Condition::ButtonReleased {
                        input: second.clone(),
                    },
                ],
            }),
        })];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 2);
        assert_eq!(model.predicates[0].kind, PredicateKind::ButtonPressed);
        assert_eq!(model.predicates[0].inputs, vec![first]);
        assert_eq!(model.predicates[1].kind, PredicateKind::ButtonReleased);
        assert_eq!(model.predicates[1].inputs, vec![second]);
    }

    #[test]
    fn duplicate_input_same_kind_dedups_to_one_chip() {
        let primary = input(0);
        let watched = button(1);
        let actions = vec![
            conditional_action(Condition::ButtonPressed {
                input: watched.clone(),
            }),
            conditional_action(Condition::ButtonPressed { input: watched }),
        ];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 1);
        assert_eq!(model.predicates[0].kind, PredicateKind::ButtonPressed);
    }

    #[test]
    fn same_input_different_kinds_renders_two_chips() {
        let primary = input(0);
        let watched = button(1);
        let actions = vec![
            conditional_action(Condition::ButtonPressed {
                input: watched.clone(),
            }),
            conditional_action(Condition::ButtonReleased { input: watched }),
        ];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 2);
        assert_eq!(model.predicates[0].kind, PredicateKind::ButtonPressed);
        assert_eq!(model.predicates[1].kind, PredicateKind::ButtonReleased);
    }

    #[test]
    fn axis_in_range_distinct_bounds_render_two_chips() {
        let primary = input(0);
        let watched = input(1);
        let actions = vec![
            conditional_action(Condition::AxisInRange {
                input: watched.clone(),
                min: 0.0,
                max: 0.5,
            }),
            conditional_action(Condition::AxisInRange {
                input: watched.clone(),
                min: 0.0,
                max: 0.5,
            }),
            conditional_action(Condition::AxisInRange {
                input: watched,
                min: 0.25,
                max: 0.75,
            }),
        ];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 2);
        assert_eq!(
            model.predicates[0].kind,
            PredicateKind::AxisInRange { min: 0.0, max: 0.5 }
        );
        assert_eq!(
            model.predicates[1].kind,
            PredicateKind::AxisInRange {
                min: 0.25,
                max: 0.75
            }
        );
    }

    #[test]
    fn axis_in_range_signed_zero_bounds_dedup_to_one_chip() {
        let primary = input(0);
        let watched = input(1);
        let actions = vec![
            conditional_action(Condition::AxisInRange {
                input: watched.clone(),
                min: -0.0,
                max: 0.5,
            }),
            conditional_action(Condition::AxisInRange {
                input: watched,
                min: 0.0,
                max: 0.5,
            }),
        ];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 1);
        assert_eq!(
            model.predicates[0].kind,
            PredicateKind::AxisInRange {
                min: -0.0,
                max: 0.5
            }
        );
    }

    #[test]
    fn hat_direction_distinct_sets_render_two_chips() {
        let primary = input(0);
        let watched = InputAddress::Bound {
            device: DeviceId("stick".to_owned()),
            input: InputId::Hat { index: 0 },
        };
        let actions = vec![
            conditional_action(Condition::HatDirection {
                input: watched.clone(),
                directions: vec![HatDirection::N],
            }),
            conditional_action(Condition::HatDirection {
                input: watched,
                directions: vec![HatDirection::N, HatDirection::E],
            }),
        ];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 2);
        assert_eq!(
            model.predicates[0].kind,
            PredicateKind::HatDirection {
                directions: vec![HatDirection::N],
            }
        );
        assert_eq!(
            model.predicates[1].kind,
            PredicateKind::HatDirection {
                directions: vec![HatDirection::N, HatDirection::E],
            }
        );
    }

    #[test]
    fn hat_direction_reordered_duplicate_sets_dedup_to_one_chip() {
        let primary = input(0);
        let watched = InputAddress::Bound {
            device: DeviceId("stick".to_owned()),
            input: InputId::Hat { index: 0 },
        };
        let actions = vec![
            conditional_action(Condition::HatDirection {
                input: watched.clone(),
                directions: vec![HatDirection::N, HatDirection::E],
            }),
            conditional_action(Condition::HatDirection {
                input: watched,
                directions: vec![HatDirection::E, HatDirection::N, HatDirection::N],
            }),
        ];

        let model = analyze_actions(&actions, &primary);

        assert_eq!(model.predicates.len(), 1);
        assert_eq!(
            model.predicates[0].kind,
            PredicateKind::HatDirection {
                directions: vec![HatDirection::N, HatDirection::E],
            }
        );
    }

    #[test]
    fn conditional_branches_record_distinct_chain_steps() {
        let primary = input(0);
        let true_output = vjoy_axis(VJoyAxis::X);
        let false_output = vjoy_axis(VJoyAxis::Y);
        let test_condition = condition();
        let condition_label = condition_label(&test_condition);
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
                    owner: model.outputs[0].owner.clone(),
                    destination: OutputDestination::VJoy(true_output),
                    chain: vec![ChainStep::Conditional {
                        condition_label: condition_label.clone(),
                        active: true,
                        branch: Branch::IfTrue,
                    }],
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                    activity: None,
                },
                OutputDescriptor {
                    owner: model.outputs[1].owner.clone(),
                    destination: OutputDestination::VJoy(false_output),
                    chain: vec![ChainStep::Conditional {
                        condition_label,
                        active: false,
                        branch: Branch::IfFalse,
                    }],
                    is_active: false,
                    polarity: AxisPolarity::Bipolar,
                    activity: None,
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
                owner: model.outputs[0].owner.clone(),
                destination: OutputDestination::VJoy(output),
                chain: Vec::new(),
                is_active: true,
                polarity: AxisPolarity::Unipolar,
                activity: None,
            }]
        );
    }

    #[test]
    fn nested_conditional_records_two_chain_steps() {
        let primary = input(0);
        let output = vjoy_axis(VJoyAxis::X);
        let outer_condition = condition();
        let inner_condition = Condition::ButtonReleased { input: button(1) };
        let outer_label = condition_label(&outer_condition);
        let inner_label = condition_label(&inner_condition);
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
                owner: model.outputs[0].owner.clone(),
                destination: OutputDestination::VJoy(output),
                chain: vec![
                    ChainStep::Conditional {
                        condition_label: outer_label,
                        active: true,
                        branch: Branch::IfTrue,
                    },
                    ChainStep::Conditional {
                        condition_label: inner_label,
                        active: true,
                        branch: Branch::IfTrue,
                    },
                ],
                is_active: true,
                polarity: AxisPolarity::Bipolar,
                activity: None,
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
        let condition_label = condition_label(&test_condition);
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
                owner: model.outputs[0].owner.clone(),
                destination: OutputDestination::VJoy(output),
                chain: vec![ChainStep::Conditional {
                    condition_label,
                    active: true,
                    branch: Branch::IfTrue,
                }],
                is_active: true,
                polarity: AxisPolarity::Bipolar,
                activity: None,
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
        assert_eq!(model.outputs.len(), 0);
    }
}
