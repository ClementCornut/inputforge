// Rust guideline compliant 2026-03-06

mod condition;
mod mapping;
mod mode_change;

pub use condition::{Condition, validate_depth};
pub use mapping::Mapping;
pub use mode_change::{CycleModes, ModeChangeStrategy};

use serde::{Deserialize, Serialize};

use crate::processing::{DeadzoneConfig, ResponseCurve};
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
        #[serde(default)]
        if_true: Vec<Action>,
        /// Both branches are always present. An empty vec encodes "do nothing
        /// when the condition is false" (semantically identical to the legacy
        /// `None` form). `#[serde(default)]` keeps backward compatibility with
        /// pre-2026-05-02 profiles that omit the field.
        #[serde(default)]
        if_false: Vec<Action>,
    },
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
            if_false: vec![Action::MapToVJoy {
                output: test_output_address(),
            }],
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"conditional\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }
}
