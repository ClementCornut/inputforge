// Rust guideline compliant 2026-03-03

mod condition;
mod merge;
#[cfg(test)]
mod test_helpers;

pub use condition::evaluate_condition;
pub use merge::merge_axes;

use crate::action::{Action, ModeChangeStrategy};
use crate::processing::invert_axis;
use crate::types::{HatDirection, InputAddress, InputId, InputValue, KeyCombo, OutputAddress};

/// Threshold above which a button's `current_value` is considered pressed.
///
/// Used when converting continuous axis-like values back to a boolean
/// press state (e.g., for `MapToVJoy` and `MapToKeyboard` actions).
const BUTTON_PRESS_THRESHOLD: f64 = 0.5;

/// Output produced by the pipeline executor.
///
/// These are transient values consumed by the output stage.
/// Not serializable -- they exist only during pipeline execution.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineOutput {
    SetAxis {
        output: OutputAddress,
        value: f64,
    },
    SetButton {
        output: OutputAddress,
        pressed: bool,
    },
    SendKey {
        key: KeyCombo,
        pressed: bool,
    },
    ChangeMode {
        strategy: ModeChangeStrategy,
    },
}

/// Read-only access to the latest input values.
///
/// Implementations provide the current state of all physical inputs,
/// used for condition evaluation and axis merging.
pub trait InputCache {
    /// Return whether the button at `address` is currently pressed.
    fn get_button(&self, address: &InputAddress) -> bool;

    /// Return the current axis value at `address`.
    fn get_axis(&self, address: &InputAddress) -> f64;

    /// Return the current hat direction at `address`.
    fn get_hat(&self, address: &InputAddress) -> HatDirection;
}

/// Mutable context carried through pipeline execution.
pub struct PipelineContext<'a> {
    pub current_value: f64,
    pub input_value: InputValue,
    pub outputs: Vec<PipelineOutput>,
    pub input_cache: &'a dyn InputCache,
}

impl std::fmt::Debug for PipelineContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineContext")
            .field("current_value", &self.current_value)
            .field("input_value", &self.input_value)
            .field("outputs", &self.outputs)
            .field("input_cache", &"<dyn InputCache>")
            .finish()
    }
}

/// Execute a sequence of actions against the given pipeline context.
pub fn execute_pipeline(actions: &[Action], ctx: &mut PipelineContext<'_>) {
    for action in actions {
        match action {
            // Processing actions transform current_value
            Action::ResponseCurve { curve } => {
                ctx.current_value = curve.evaluate(ctx.current_value);
            }
            Action::Deadzone { config } => {
                ctx.current_value = config.apply(ctx.current_value);
            }
            Action::Invert => match &ctx.input_value {
                InputValue::Button { .. } => {
                    ctx.current_value = if ctx.current_value > BUTTON_PRESS_THRESHOLD {
                        0.0
                    } else {
                        1.0
                    };
                }
                InputValue::Hat { .. } => {
                    tracing::debug!("hat inversion is a no-op");
                }
                InputValue::Axis { .. } => {
                    ctx.current_value = invert_axis(ctx.current_value);
                }
            },

            // Output actions push to ctx.outputs
            Action::MapToVJoy { output } => match &ctx.input_value {
                InputValue::Axis { .. } => {
                    ctx.outputs.push(PipelineOutput::SetAxis {
                        output: output.clone(),
                        value: ctx.current_value,
                    });
                }
                InputValue::Button { .. } => {
                    ctx.outputs.push(PipelineOutput::SetButton {
                        output: output.clone(),
                        pressed: ctx.current_value > BUTTON_PRESS_THRESHOLD,
                    });
                }
                InputValue::Hat { .. } => {
                    tracing::debug!("hat-to-vJoy mapping not yet implemented");
                }
            },
            Action::MapToKeyboard { key } => match &ctx.input_value {
                InputValue::Hat { .. } => {
                    tracing::debug!("hat-to-keyboard mapping not yet implemented");
                }
                _ => {
                    ctx.outputs.push(PipelineOutput::SendKey {
                        key: key.clone(),
                        pressed: ctx.current_value > BUTTON_PRESS_THRESHOLD,
                    });
                }
            },
            Action::MergeAxis {
                second_input,
                operation,
            } => {
                let second = ctx.input_cache.get_axis(second_input);
                ctx.current_value = merge_axes(ctx.current_value, second, *operation);
            }

            // Control flow
            Action::ChangeMode { strategy } => {
                ctx.outputs.push(PipelineOutput::ChangeMode {
                    strategy: strategy.clone(),
                });
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                if evaluate_condition(condition, ctx.input_cache) {
                    execute_pipeline(if_true, ctx);
                } else if let Some(false_actions) = if_false {
                    execute_pipeline(false_actions, ctx);
                }
            }
        }
    }
}

/// Re-run a partial action pipeline against a snapshot and return the
/// projected `InputValue` at `stop_at`.
///
/// `stop_at = 0` returns the unprocessed input read from `state.input_cache`
/// at `primary`. `stop_at >= actions.len()` runs the full pipeline.
///
/// Read-only; never dispatches commands. Used by F10's live-tracking dot
/// (and F9's live-readout OUT bar) without duplicating pipeline evaluation
/// in the GUI.
#[must_use]
pub fn evaluate_actions_through(
    actions: &[Action],
    state: &crate::state::AppState,
    primary: &InputAddress,
    stop_at: usize,
) -> InputValue {
    let stop = stop_at.min(actions.len());

    // Discriminate variant from the address; read via the InputCache trait.
    // Returns the cache's default for missing entries (axis: 0.0, button: false,
    // hat: HatDirection::Center): same convention as direct trait reads.
    let input_value = match &primary.input {
        InputId::Axis { .. } => InputValue::Axis {
            value: crate::types::AxisValue::new(state.input_cache.get_axis(primary)),
        },
        InputId::Button { .. } => InputValue::Button {
            pressed: state.input_cache.get_button(primary),
        },
        InputId::Hat { .. } => InputValue::Hat {
            direction: state.input_cache.get_hat(primary),
        },
    };

    let current_value: f64 = match &input_value {
        InputValue::Axis { value } => value.value(),
        InputValue::Button { pressed } => {
            if *pressed {
                1.0
            } else {
                0.0
            }
        }
        InputValue::Hat { .. } => 0.0,
    };

    let mut ctx = PipelineContext {
        current_value,
        input_value: input_value.clone(),
        outputs: Vec::new(),
        input_cache: &state.input_cache,
    };

    execute_pipeline(&actions[..stop], &mut ctx);

    match input_value {
        InputValue::Axis { .. } => InputValue::Axis {
            value: crate::types::AxisValue::new(ctx.current_value),
        },
        InputValue::Button { .. } => InputValue::Button {
            pressed: ctx.current_value > BUTTON_PRESS_THRESHOLD,
        },
        // Hats: pipeline evaluation does not modify direction; the cached
        // direction reads through unchanged.
        InputValue::Hat { direction } => InputValue::Hat { direction },
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::{MockCache, button_input_address};
    use super::*;
    use crate::action::Condition;
    use crate::processing::{DeadzoneConfig, ResponseCurve};
    use crate::types::{AxisValue, DeviceId, KeyModifier, MergeOp, OutputId, VJoyAxis};

    const TOLERANCE: f64 = 1e-6;

    fn test_output() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    fn button_output() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Button { id: 1 },
        }
    }

    fn axis_ctx(cache: &dyn InputCache, value: f64) -> PipelineContext<'_> {
        PipelineContext {
            current_value: value,
            input_value: InputValue::Axis {
                value: AxisValue::new(value),
            },
            outputs: Vec::new(),
            input_cache: cache,
        }
    }

    fn button_ctx(cache: &dyn InputCache, pressed: bool) -> PipelineContext<'_> {
        PipelineContext {
            current_value: if pressed { 1.0 } else { 0.0 },
            input_value: InputValue::Button { pressed },
            outputs: Vec::new(),
            input_cache: cache,
        }
    }

    // -- Empty pipeline -------------------------------------------------------

    #[test]
    fn empty_pipeline_no_output() {
        let cache = MockCache::new();
        let mut ctx = axis_ctx(&cache, 0.5);
        execute_pipeline(&[], &mut ctx);
        assert!(ctx.outputs.is_empty());
    }

    // -- Axis passthrough -----------------------------------------------------

    #[test]
    fn axis_passthrough_map_to_vjoy() {
        let cache = MockCache::new();
        let mut ctx = axis_ctx(&cache, 0.75);
        let actions = [Action::MapToVJoy {
            output: test_output(),
        }];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(ctx.outputs.len(), 1);
        assert_eq!(
            ctx.outputs[0],
            PipelineOutput::SetAxis {
                output: test_output(),
                value: 0.75,
            }
        );
    }

    // -- Button passthrough ---------------------------------------------------

    #[test]
    fn button_passthrough_set_button() {
        let cache = MockCache::new();
        let mut ctx = button_ctx(&cache, true);
        let actions = [Action::MapToVJoy {
            output: button_output(),
        }];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(ctx.outputs.len(), 1);
        assert_eq!(
            ctx.outputs[0],
            PipelineOutput::SetButton {
                output: button_output(),
                pressed: true,
            }
        );
    }

    // -- Hat input passthrough ------------------------------------------------

    #[test]
    fn hat_input_map_to_vjoy_no_output() {
        let cache = MockCache::new();
        let mut ctx = PipelineContext {
            current_value: 0.0,
            input_value: InputValue::Hat {
                direction: HatDirection::N,
            },
            outputs: Vec::new(),
            input_cache: &cache,
        };
        let actions = [Action::MapToVJoy {
            output: test_output(),
        }];
        execute_pipeline(&actions, &mut ctx);
        assert!(ctx.outputs.is_empty());
    }

    // -- Debug impl -----------------------------------------------------------

    #[test]
    fn pipeline_context_debug_formats() {
        let cache = MockCache::new();
        let ctx = axis_ctx(&cache, 0.5);
        let debug = format!("{ctx:?}");
        assert!(debug.contains("PipelineContext"));
        assert!(debug.contains("current_value"));
        assert!(debug.contains("<dyn InputCache>"));
    }

    // -- Invert + MapToVJoy ---------------------------------------------------

    #[test]
    fn invert_button_pressed_to_released() {
        let cache = MockCache::new();
        let mut ctx = button_ctx(&cache, true);
        let actions = [
            Action::Invert,
            Action::MapToVJoy {
                output: button_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(
            ctx.outputs[0],
            PipelineOutput::SetButton {
                output: button_output(),
                pressed: false,
            }
        );
    }

    #[test]
    fn invert_button_released_to_pressed() {
        let cache = MockCache::new();
        let mut ctx = button_ctx(&cache, false);
        let actions = [
            Action::Invert,
            Action::MapToVJoy {
                output: button_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(
            ctx.outputs[0],
            PipelineOutput::SetButton {
                output: button_output(),
                pressed: true,
            }
        );
    }

    #[test]
    fn invert_then_map_negates_axis() {
        let cache = MockCache::new();
        let mut ctx = axis_ctx(&cache, 0.5);
        let actions = [
            Action::Invert,
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(
            ctx.outputs[0],
            PipelineOutput::SetAxis {
                output: test_output(),
                value: -0.5,
            }
        );
    }

    // -- Deadzone + MapToVJoy -------------------------------------------------

    #[test]
    fn deadzone_center_becomes_zero() {
        let cache = MockCache::new();
        let mut ctx = axis_ctx(&cache, 0.02);
        let actions = [
            Action::Deadzone {
                config: DeadzoneConfig::new(-1.0, -0.05, 0.05, 1.0).unwrap(),
            },
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(value.abs() < TOLERANCE, "expected 0, got {value}");
        } else {
            panic!("expected SetAxis");
        }
    }

    // -- ResponseCurve + MapToVJoy --------------------------------------------

    #[test]
    fn response_curve_identity_passthrough() {
        let cache = MockCache::new();
        let mut ctx = axis_ctx(&cache, 0.6);
        let actions = [
            Action::ResponseCurve {
                curve: ResponseCurve::piecewise_linear(
                    vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
                    false,
                )
                .unwrap(),
            },
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - 0.6).abs() < TOLERANCE,
                "expected 0.6, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    // -- Conditional ----------------------------------------------------------

    #[test]
    fn conditional_true_branch() {
        let mut cache = MockCache::new();
        cache.buttons.insert(button_input_address(), true);
        let mut ctx = axis_ctx(&cache, 0.5);
        let actions = [Action::Conditional {
            condition: Condition::ButtonPressed {
                input: button_input_address(),
            },
            if_true: vec![
                Action::Invert,
                Action::MapToVJoy {
                    output: test_output(),
                },
            ],
            if_false: Some(vec![Action::MapToVJoy {
                output: test_output(),
            }]),
        }];
        execute_pipeline(&actions, &mut ctx);
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - (-0.5)).abs() < TOLERANCE,
                "expected -0.5, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    #[test]
    fn conditional_false_branch() {
        let cache = MockCache::new(); // button defaults to false
        let mut ctx = axis_ctx(&cache, 0.5);
        let actions = [Action::Conditional {
            condition: Condition::ButtonPressed {
                input: button_input_address(),
            },
            if_true: vec![
                Action::Invert,
                Action::MapToVJoy {
                    output: test_output(),
                },
            ],
            if_false: Some(vec![Action::MapToVJoy {
                output: test_output(),
            }]),
        }];
        execute_pipeline(&actions, &mut ctx);
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - 0.5).abs() < TOLERANCE,
                "expected 0.5, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    #[test]
    fn conditional_no_else_false_condition_no_output() {
        let cache = MockCache::new();
        let mut ctx = axis_ctx(&cache, 0.5);
        let actions = [Action::Conditional {
            condition: Condition::ButtonPressed {
                input: button_input_address(),
            },
            if_true: vec![Action::MapToVJoy {
                output: test_output(),
            }],
            if_false: None,
        }];
        execute_pipeline(&actions, &mut ctx);
        assert!(ctx.outputs.is_empty());
    }

    // -- MergeAxis ------------------------------------------------------------

    #[test]
    fn merge_axis_bidirectional() {
        let mut cache = MockCache::new();
        let second_addr = InputAddress {
            device: DeviceId("pedals".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        cache.axes.insert(second_addr.clone(), 0.3);
        let mut ctx = axis_ctx(&cache, 0.8);
        let actions = [
            Action::MergeAxis {
                second_input: second_addr,
                operation: MergeOp::Bidirectional,
            },
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - 0.5).abs() < TOLERANCE,
                "expected 0.5, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    #[test]
    fn merge_axis_average() {
        let mut cache = MockCache::new();
        let second_addr = InputAddress {
            device: DeviceId("pedals".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        cache.axes.insert(second_addr.clone(), 0.4);
        let mut ctx = axis_ctx(&cache, 0.8);
        let actions = [
            Action::MergeAxis {
                second_input: second_addr,
                operation: MergeOp::Average,
            },
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - 0.6).abs() < TOLERANCE,
                "expected 0.6, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    #[test]
    fn merge_axis_maximum() {
        let mut cache = MockCache::new();
        let second_addr = InputAddress {
            device: DeviceId("pedals".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        cache.axes.insert(second_addr.clone(), -0.9);
        let mut ctx = axis_ctx(&cache, 0.3);
        let actions = [
            Action::MergeAxis {
                second_input: second_addr,
                operation: MergeOp::Maximum,
            },
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        // |-0.9| > |0.3|, so the result is -0.9
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - (-0.9)).abs() < TOLERANCE,
                "expected -0.9, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    #[test]
    fn merge_axis_maximum_first_larger_abs() {
        let mut cache = MockCache::new();
        let second_addr = InputAddress {
            device: DeviceId("pedals".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        cache.axes.insert(second_addr.clone(), 0.3);
        let mut ctx = axis_ctx(&cache, -0.8);
        let actions = [
            Action::MergeAxis {
                second_input: second_addr,
                operation: MergeOp::Maximum,
            },
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        // |-0.8| > |0.3|, so the result is -0.8 (first wins)
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - (-0.8)).abs() < TOLERANCE,
                "expected -0.8, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    // -- MapToKeyboard --------------------------------------------------------

    #[test]
    fn map_to_keyboard_with_button() {
        let cache = MockCache::new();
        let mut ctx = button_ctx(&cache, true);
        let key = KeyCombo {
            key: "Space".to_owned(),
            modifiers: vec![KeyModifier::Ctrl],
        };
        let actions = [Action::MapToKeyboard { key: key.clone() }];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(ctx.outputs.len(), 1);
        assert_eq!(
            ctx.outputs[0],
            PipelineOutput::SendKey { key, pressed: true }
        );
    }

    // -- ChangeMode -----------------------------------------------------------

    #[test]
    fn change_mode_output() {
        let cache = MockCache::new();
        let mut ctx = button_ctx(&cache, true);
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "combat".to_owned(),
        };
        let actions = [Action::ChangeMode {
            strategy: strategy.clone(),
        }];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(ctx.outputs.len(), 1);
        assert_eq!(ctx.outputs[0], PipelineOutput::ChangeMode { strategy });
    }

    // -- Multiple outputs -----------------------------------------------------

    #[test]
    fn multiple_outputs_from_same_pipeline() {
        let cache = MockCache::new();
        let mut ctx = axis_ctx(&cache, 0.5);
        let output_x = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        };
        let output_y = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::Y },
        };
        let actions = [
            Action::MapToVJoy {
                output: output_x.clone(),
            },
            Action::MapToVJoy {
                output: output_y.clone(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(ctx.outputs.len(), 2);
        assert_eq!(
            ctx.outputs[0],
            PipelineOutput::SetAxis {
                output: output_x,
                value: 0.5,
            }
        );
        assert_eq!(
            ctx.outputs[1],
            PipelineOutput::SetAxis {
                output: output_y,
                value: 0.5,
            }
        );
    }

    // -- Full processing chain ------------------------------------------------

    #[test]
    fn full_chain_deadzone_curve_invert_map() {
        let cache = MockCache::new();
        // Use a no-deadzone config (center at 0, no dead band)
        let deadzone = DeadzoneConfig::new(-1.0, 0.0, 0.0, 1.0).unwrap();
        // Pre-calibrated value 0.5 → deadzone → 0.5 → identity curve → 0.5 → invert → -0.5
        let mut ctx = axis_ctx(&cache, 0.5);
        let actions = [
            Action::Deadzone { config: deadzone },
            Action::ResponseCurve {
                curve: ResponseCurve::piecewise_linear(
                    vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
                    false,
                )
                .unwrap(),
            },
            Action::Invert,
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        assert_eq!(ctx.outputs.len(), 1);
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - (-0.5)).abs() < TOLERANCE,
                "expected -0.5, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    // -- Pedal merge integration: full pipeline --------------------------------

    #[test]
    fn pedal_merge_full_pipeline() {
        // Simulate two pre-calibrated pedal axes going through deadzone + merge + map.
        // Left pedal pre-calibrated value: -0.5 (half depressed)
        // Right pedal cached at -0.25
        //
        // Pipeline for left pedal:
        //   deadzone (trivial, pass-through) → -0.5
        //   merge bidirectional with right pedal → -0.5 - (-0.25) = -0.25
        //   map to vJoy Rz axis

        let mut cache = MockCache::new();
        let right_pedal = InputAddress {
            device: DeviceId("pedals".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        cache.axes.insert(right_pedal.clone(), -0.25);

        let deadzone = DeadzoneConfig::new(-1.0, 0.0, 0.0, 1.0).unwrap();
        let output = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::Rz },
        };

        let mut ctx = axis_ctx(&cache, -0.5);

        let actions = [
            Action::Deadzone { config: deadzone },
            Action::MergeAxis {
                second_input: right_pedal,
                operation: MergeOp::Bidirectional,
            },
            Action::MapToVJoy {
                output: output.clone(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);

        assert_eq!(ctx.outputs.len(), 1);
        if let PipelineOutput::SetAxis { value, output: out } = &ctx.outputs[0] {
            assert_eq!(*out, output);
            assert!(
                (*value - (-0.25)).abs() < TOLERANCE,
                "expected -0.25, got {value}"
            );
        } else {
            panic!("expected SetAxis");
        }
    }

    // -- Hat + MapToKeyboard --------------------------------------------------

    #[test]
    fn hat_input_map_to_keyboard_no_output() {
        let cache = MockCache::new();
        let mut ctx = PipelineContext {
            current_value: 0.0,
            input_value: InputValue::Hat {
                direction: HatDirection::N,
            },
            outputs: Vec::new(),
            input_cache: &cache,
        };
        let key = KeyCombo {
            key: "Space".to_owned(),
            modifiers: vec![],
        };
        let actions = [Action::MapToKeyboard { key }];
        execute_pipeline(&actions, &mut ctx);
        assert!(ctx.outputs.is_empty());
    }

    // -- evaluate_actions_through ---------------------------------------------

    use crate::state::AppState;

    fn axis_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        }
    }

    fn hat_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Hat { index: 0 },
        }
    }

    #[test]
    fn evaluate_actions_through_zero_returns_input_untouched() {
        let mut state = AppState::new();
        let addr = axis_input_address();
        state.input_cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );

        let actions = [Action::Invert];
        let out = evaluate_actions_through(&actions, &state, &addr, 0);

        match out {
            InputValue::Axis { value } => {
                assert!((value.value() - 0.5).abs() < TOLERANCE);
            }
            other => panic!("expected Axis, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_actions_through_full_runs_entire_pipeline() {
        let mut state = AppState::new();
        let addr = axis_input_address();
        state.input_cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );

        let actions = [Action::Invert];
        let out = evaluate_actions_through(&actions, &state, &addr, actions.len());

        match out {
            InputValue::Axis { value } => {
                assert!((value.value() - (-0.5)).abs() < TOLERANCE);
            }
            other => panic!("expected Axis, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_actions_through_partial_runs_subset() {
        let mut state = AppState::new();
        let addr = axis_input_address();
        state.input_cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );

        // Two Inverts cancel; stop_at=1 runs only the first.
        let actions = [Action::Invert, Action::Invert];
        let out = evaluate_actions_through(&actions, &state, &addr, 1);
        match out {
            InputValue::Axis { value } => {
                assert!((value.value() - (-0.5)).abs() < TOLERANCE);
            }
            other => panic!("expected Axis, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_actions_through_stop_at_overflow_clamps() {
        let mut state = AppState::new();
        let addr = axis_input_address();
        state.input_cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );

        let actions = [Action::Invert];
        // stop_at = 99 with 1 action; clamps to 1.
        let out = evaluate_actions_through(&actions, &state, &addr, 99);
        match out {
            InputValue::Axis { value } => {
                assert!((value.value() - (-0.5)).abs() < TOLERANCE);
            }
            other => panic!("expected Axis, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_actions_through_button_pipeline() {
        let mut state = AppState::new();
        let addr = button_input_address();
        state
            .input_cache
            .update(&addr, &InputValue::Button { pressed: true });

        let actions = [Action::Invert];
        let out = evaluate_actions_through(&actions, &state, &addr, 1);
        match out {
            InputValue::Button { pressed } => assert!(!pressed, "Invert should flip true to false"),
            other => panic!("expected Button, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_actions_through_unknown_input_returns_zero_axis() {
        // Defensive: if the address is missing from the cache, the helper
        // synthesizes an Axis(0.0) baseline (same convention used by
        // InputCache trait readers).
        let state = AppState::new();
        let addr = axis_input_address();
        let actions: [Action; 0] = [];
        let out = evaluate_actions_through(&actions, &state, &addr, 0);
        match out {
            InputValue::Axis { value } => {
                assert!(value.value().abs() < TOLERANCE);
            }
            other => panic!("expected Axis, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_actions_through_hat_pipeline_passes_direction_through() {
        // Hats: ctx.current_value is meaningless; the helper preserves the
        // original direction read from input_cache regardless of stop_at.
        let mut state = AppState::new();
        let addr = hat_input_address();
        state.input_cache.update(
            &addr,
            &InputValue::Hat {
                direction: HatDirection::NE,
            },
        );

        let actions: [Action; 0] = [];
        let out = evaluate_actions_through(&actions, &state, &addr, 0);
        match out {
            InputValue::Hat { direction } => assert_eq!(direction, HatDirection::NE),
            other => panic!("expected Hat, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_actions_through_partial_one_before_end_runs_subset() {
        // Boundary: stop_at = actions.len() - 1 must run all but the last action.
        // Distinct from the existing partial test (which uses stop_at = 1 with len 2).
        let mut state = AppState::new();
        let addr = axis_input_address();
        state.input_cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );

        // Three Inverts; stop_at = 2 leaves one un-applied -> result inverted twice (= 0.5).
        let actions = [Action::Invert, Action::Invert, Action::Invert];
        let out = evaluate_actions_through(&actions, &state, &addr, actions.len() - 1);
        match out {
            InputValue::Axis { value } => {
                assert!((value.value() - 0.5).abs() < TOLERANCE);
            }
            other => panic!("expected Axis, got {other:?}"),
        }
    }
}
