// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use crate::processing::{Calibration, DeadzoneConfig, ResponseCurve};
use crate::types::{InputAddress, KeyCombo, MergeOp, OutputAddress};

/// An action in the input processing pipeline.
///
/// Actions fall into three categories:
/// - **Processing:** Transform the current value (e.g., deadzone, invert).
/// - **Output:** Produce a side effect (e.g., map to vJoy, send a key).
/// - **Control flow:** Branch or change modes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    // Processing (transform current_value)
    ResponseCurve {
        curve: ResponseCurve,
    },
    Deadzone {
        config: DeadzoneConfig,
    },
    Calibrate {
        config: Calibration,
    },
    Invert,

    // Output (produce side effects)
    #[serde(rename = "map_to_vjoy")]
    MapToVJoy {
        output: OutputAddress,
    },
    MapToKeyboard {
        key: KeyCombo,
    },
    MergeAxis {
        second_input: InputAddress,
        operation: MergeOp,
    },

    // Control flow
    ChangeMode {
        strategy: ModeChangeStrategy,
    },
    Conditional {
        condition: Condition,
        if_true: Vec<Action>,
        if_false: Option<Vec<Action>>,
    },
}

/// Strategy for changing the active input mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum ModeChangeStrategy {
    SwitchTo { mode: String },
    Temporary { mode: String },
    Previous,
    Cycle { modes: Vec<String> },
}

/// A condition that can be evaluated against the current input state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    ButtonPressed { input: InputAddress },
    ButtonReleased { input: InputAddress },
    AxisInRange { input: InputAddress, min: f64, max: f64 },
    All { conditions: Vec<Condition> },
    Any { conditions: Vec<Condition> },
    Not { condition: Box<Condition> },
}

/// A mapping from a physical input to a sequence of processing actions.
///
/// Placed in this module (not `types/mapping.rs`) because it references [`Action`],
/// which depends on `processing/`. Placing it there would create a circular dependency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mapping {
    pub input: InputAddress,
    pub mode: String,
    pub actions: Vec<Action>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, InputId, OutputId, VJoyAxis};

    fn test_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

    fn test_output_address() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    #[test]
    fn action_invert_serde_roundtrip() {
        let action = Action::Invert;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"invert\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn action_deadzone_serde_roundtrip() {
        let action = Action::Deadzone {
            config: DeadzoneConfig::default(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"deadzone\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn action_map_to_vjoy_serde_roundtrip() {
        let action = Action::MapToVJoy {
            output: test_output_address(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"map_to_vjoy\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn action_conditional_serde_roundtrip() {
        let action = Action::Conditional {
            condition: Condition::ButtonPressed {
                input: test_input_address(),
            },
            if_true: vec![Action::Invert],
            if_false: Some(vec![Action::MapToVJoy {
                output: test_output_address(),
            }]),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"conditional\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn condition_button_pressed_serde_roundtrip() {
        let cond = Condition::ButtonPressed {
            input: test_input_address(),
        };
        let json = serde_json::to_string(&cond).unwrap();
        assert!(json.contains("\"type\":\"button_pressed\""));
        let back: Condition = serde_json::from_str(&json).unwrap();
        assert_eq!(cond, back);
    }

    #[test]
    fn condition_not_serde_roundtrip() {
        let cond = Condition::Not {
            condition: Box::new(Condition::ButtonReleased {
                input: test_input_address(),
            }),
        };
        let json = serde_json::to_string(&cond).unwrap();
        assert!(json.contains("\"type\":\"not\""));
        let back: Condition = serde_json::from_str(&json).unwrap();
        assert_eq!(cond, back);
    }

    #[test]
    fn condition_axis_in_range_serde_roundtrip() {
        let cond = Condition::AxisInRange {
            input: InputAddress {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            min: -0.5,
            max: 0.5,
        };
        let json = serde_json::to_string(&cond).unwrap();
        let back: Condition = serde_json::from_str(&json).unwrap();
        assert_eq!(cond, back);
    }

    #[test]
    fn condition_all_serde_roundtrip() {
        let cond = Condition::All {
            conditions: vec![
                Condition::ButtonPressed {
                    input: test_input_address(),
                },
                Condition::ButtonReleased {
                    input: test_input_address(),
                },
            ],
        };
        let json = serde_json::to_string(&cond).unwrap();
        let back: Condition = serde_json::from_str(&json).unwrap();
        assert_eq!(cond, back);
    }

    #[test]
    fn mode_change_strategy_switch_to_serde_roundtrip() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "combat".to_owned(),
        };
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy\":\"switch_to\""));
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }

    #[test]
    fn mode_change_strategy_cycle_serde_roundtrip() {
        let strategy = ModeChangeStrategy::Cycle {
            modes: vec!["mode_a".to_owned(), "mode_b".to_owned()],
        };
        let json = serde_json::to_string(&strategy).unwrap();
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }

    #[test]
    fn mode_change_strategy_previous_serde_roundtrip() {
        let strategy = ModeChangeStrategy::Previous;
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy\":\"previous\""));
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }

    #[test]
    fn mapping_serde_roundtrip() {
        let mapping = Mapping {
            input: test_input_address(),
            mode: "default".to_owned(),
            actions: vec![
                Action::Deadzone {
                    config: DeadzoneConfig::default(),
                },
                Action::Invert,
                Action::MapToVJoy {
                    output: test_output_address(),
                },
            ],
        };
        let json = serde_json::to_string(&mapping).unwrap();
        let back: Mapping = serde_json::from_str(&json).unwrap();
        assert_eq!(mapping, back);
    }
}
