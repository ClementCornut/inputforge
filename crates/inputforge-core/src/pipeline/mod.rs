// Rust guideline compliant 2026-03-02

mod condition;
mod merge;

pub use condition::evaluate_condition;
pub use merge::merge_axes;

use crate::action::{Action, ModeChangeStrategy};
use crate::processing::{invert_axis, invert_button};
use crate::types::{InputAddress, InputValue, KeyCombo, OutputAddress};

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
    fn get_hat(&self, address: &InputAddress) -> crate::types::HatDirection;
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
            Action::Calibrate { config } => {
                ctx.current_value = config.apply(ctx.current_value);
            }
            Action::Invert => match &ctx.input_value {
                InputValue::Button { pressed } => {
                    ctx.current_value = if invert_button(*pressed) { 1.0 } else { 0.0 };
                }
                _ => {
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
                        pressed: ctx.current_value > 0.5,
                    });
                }
                InputValue::Hat { .. } => {}
            },
            Action::MapToKeyboard { key } => match &ctx.input_value {
                InputValue::Hat { .. } => {}
                _ => {
                    ctx.outputs.push(PipelineOutput::SendKey {
                        key: key.clone(),
                        pressed: ctx.current_value > 0.5,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::action::Condition;
    use crate::processing::{Calibration, DeadzoneConfig, ResponseCurve};
    use crate::types::{AxisValue, DeviceId, InputId, KeyModifier, MergeOp, OutputId, VJoyAxis};

    const TOLERANCE: f64 = 1e-6;

    struct MockCache {
        buttons: HashMap<InputAddress, bool>,
        axes: HashMap<InputAddress, f64>,
    }

    impl MockCache {
        fn new() -> Self {
            Self {
                buttons: HashMap::new(),
                axes: HashMap::new(),
            }
        }
    }

    impl InputCache for MockCache {
        fn get_button(&self, address: &InputAddress) -> bool {
            self.buttons.get(address).copied().unwrap_or(false)
        }

        fn get_axis(&self, address: &InputAddress) -> f64 {
            self.axes.get(address).copied().unwrap_or(0.0)
        }

        fn get_hat(&self, address: &InputAddress) -> crate::types::HatDirection {
            crate::types::HatDirection::Center
        }
    }

    fn button_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("stick-1".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

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
                direction: crate::types::HatDirection::N,
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
                config: DeadzoneConfig::default(),
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

    // -- Calibrate + MapToVJoy ------------------------------------------------

    #[test]
    fn calibrate_normalizes_raw_value() {
        let cache = MockCache::new();
        let mut ctx = axis_ctx(&cache, 32767.0);
        let actions = [
            Action::Calibrate {
                config: Calibration::new(-32768.0, -100.0, 100.0, 32767.0, true).unwrap(),
            },
            Action::MapToVJoy {
                output: test_output(),
            },
        ];
        execute_pipeline(&actions, &mut ctx);
        if let PipelineOutput::SetAxis { value, .. } = &ctx.outputs[0] {
            assert!(
                (*value - 1.0).abs() < TOLERANCE,
                "expected 1.0, got {value}"
            );
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
    fn full_chain_calibrate_deadzone_curve_invert_map() {
        let cache = MockCache::new();
        // Use a no-deadzone config (center at 0, no dead band)
        let deadzone = DeadzoneConfig::new(-1.0, 0.0, 0.0, 1.0).unwrap();
        let calibration = Calibration::new(-100.0, 0.0, 0.0, 100.0, true).unwrap();
        // Raw value 50.0 → calibrate → 0.5 → deadzone → 0.5 → identity curve → 0.5 → invert → -0.5
        let mut ctx = PipelineContext {
            current_value: 50.0,
            input_value: InputValue::Axis {
                value: AxisValue::raw(50.0),
            },
            outputs: Vec::new(),
            input_cache: &cache,
        };
        let actions = [
            Action::Calibrate {
                config: calibration,
            },
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
        // Simulate two pedal axes going through calibration + deadzone + merge + map.
        // Left pedal raw value: -16384 (half depressed)
        // Right pedal raw value cached as calibrated -0.25
        //
        // Pipeline for left pedal:
        //   calibrate(-32768..32767) -> -0.5
        //   deadzone (trivial, pass-through)
        //   merge bidirectional with right pedal -> (-0.5) - (-0.25) = -0.25
        //   map to vJoy X axis

        let mut cache = MockCache::new();
        let right_pedal = InputAddress {
            device: DeviceId("pedals".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        // Right pedal already calibrated in cache at -0.25
        cache.axes.insert(right_pedal.clone(), -0.25);

        let calibration = Calibration::new(-32768.0, -100.0, 100.0, 32767.0, true).unwrap();
        let deadzone = DeadzoneConfig::new(-1.0, 0.0, 0.0, 1.0).unwrap();
        let output = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::Rz },
        };

        let mut ctx = PipelineContext {
            current_value: -16384.0,
            input_value: InputValue::Axis {
                value: AxisValue::raw(-16384.0),
            },
            outputs: Vec::new(),
            input_cache: &cache,
        };

        let actions = [
            Action::Calibrate {
                config: calibration,
            },
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
            // calibrate: -16384 / 32767 ~= -0.5000..., merged: -0.5 - (-0.25) = -0.25
            assert!(
                (*value - (-0.25)).abs() < 0.01,
                "expected ~-0.25, got {value}"
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
                direction: crate::types::HatDirection::N,
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
}
