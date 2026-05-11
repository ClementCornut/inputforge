// Rust guideline compliant 2026-03-06

mod bulk;
mod condition;
mod mapping;
mod mode_change;

pub use bulk::BulkMapEntry;
pub use condition::{Condition, validate_depth};
pub use mapping::Mapping;
pub use mode_change::ModeChangeStrategy;

use serde::{Deserialize, Serialize};

use crate::processing::{DeadzoneConfig, ResponseCurve};
use crate::types::{InputAddress, KeyCombo, MergeOp, OutputAddress};

/// An action in the input processing pipeline.
///
/// Actions fall into three categories:
/// - **Processing:** Transform the current value (e.g., deadzone, invert).
/// - **Output:** Produce a side effect (e.g., map to vJoy, send a key).
/// - **Control flow:** Branch or change modes.
fn default_output_behavior() -> OutputBehavior {
    OutputBehavior::Hold
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputBehavior {
    Hold,
    Pulse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseTarget {
    LeftButton,
    RightButton,
    MiddleButton,
    BackButton,
    ForwardButton,
    WheelUp,
    WheelDown,
}

impl MouseTarget {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::LeftButton => "Left click",
            Self::RightButton => "Right click",
            Self::MiddleButton => "Middle click",
            Self::BackButton => "Back button",
            Self::ForwardButton => "Forward button",
            Self::WheelUp => "Wheel up",
            Self::WheelDown => "Wheel down",
        }
    }

    #[must_use]
    pub const fn is_wheel(self) -> bool {
        matches!(self, Self::WheelUp | Self::WheelDown)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MapToMouseAction {
    target: MouseTarget,
    #[serde(default = "default_output_behavior")]
    behavior: OutputBehavior,
}

impl MapToMouseAction {
    fn into_parts(self) -> (MouseTarget, OutputBehavior) {
        let behavior = if self.target.is_wheel() {
            OutputBehavior::Pulse
        } else {
            self.behavior
        };
        (self.target, behavior)
    }
}

#[derive(Debug, Clone, PartialEq)]
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
    MapToVJoy {
        output: OutputAddress,
    },
    MapToKeyboard {
        key: KeyCombo,
        behavior: OutputBehavior,
    },
    MapToMouse {
        target: MouseTarget,
        behavior: OutputBehavior,
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
        /// Both branches are always present. An empty vec encodes "do nothing
        /// when the condition is false" (semantically identical to the legacy
        /// `None` form). `#[serde(default)]` keeps backward compatibility with
        /// pre-2026-05-02 profiles that omit the field.
        if_false: Vec<Action>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ActionSerde {
    ResponseCurve {
        curve: ResponseCurve,
    },
    Deadzone {
        config: DeadzoneConfig,
    },
    Invert,
    #[serde(rename = "map_to_vjoy")]
    MapToVJoy {
        output: OutputAddress,
    },
    MapToKeyboard {
        key: KeyCombo,
        #[serde(default = "default_output_behavior")]
        behavior: OutputBehavior,
    },
    #[serde(rename = "map_to_mouse")]
    MapToMouse {
        target: MouseTarget,
        #[serde(default = "default_output_behavior")]
        behavior: OutputBehavior,
    },
    MergeAxis {
        second_input: InputAddress,
        operation: MergeOp,
    },
    ChangeMode {
        strategy: ModeChangeStrategy,
    },
    Conditional {
        condition: Condition,
        #[serde(default)]
        if_true: Vec<Action>,
        #[serde(default)]
        if_false: Vec<Action>,
    },
}

impl From<Action> for ActionSerde {
    fn from(action: Action) -> Self {
        match action {
            Action::ResponseCurve { curve } => Self::ResponseCurve { curve },
            Action::Deadzone { config } => Self::Deadzone { config },
            Action::Invert => Self::Invert,
            Action::MapToVJoy { output } => Self::MapToVJoy { output },
            Action::MapToKeyboard { key, behavior } => Self::MapToKeyboard { key, behavior },
            Action::MapToMouse { target, behavior } => {
                let behavior = if target.is_wheel() {
                    OutputBehavior::Pulse
                } else {
                    behavior
                };
                Self::MapToMouse { target, behavior }
            }
            Action::MergeAxis {
                second_input,
                operation,
            } => Self::MergeAxis {
                second_input,
                operation,
            },
            Action::ChangeMode { strategy } => Self::ChangeMode { strategy },
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => Self::Conditional {
                condition,
                if_true,
                if_false,
            },
        }
    }
}

impl From<ActionSerde> for Action {
    fn from(action: ActionSerde) -> Self {
        match action {
            ActionSerde::ResponseCurve { curve } => Self::ResponseCurve { curve },
            ActionSerde::Deadzone { config } => Self::Deadzone { config },
            ActionSerde::Invert => Self::Invert,
            ActionSerde::MapToVJoy { output } => Self::MapToVJoy { output },
            ActionSerde::MapToKeyboard { key, behavior } => Self::MapToKeyboard { key, behavior },
            ActionSerde::MapToMouse { target, behavior } => {
                let (target, behavior) = MapToMouseAction { target, behavior }.into_parts();
                Self::MapToMouse { target, behavior }
            }
            ActionSerde::MergeAxis {
                second_input,
                operation,
            } => Self::MergeAxis {
                second_input,
                operation,
            },
            ActionSerde::ChangeMode { strategy } => Self::ChangeMode { strategy },
            ActionSerde::Conditional {
                condition,
                if_true,
                if_false,
            } => Self::Conditional {
                condition,
                if_true,
                if_false,
            },
        }
    }
}

impl Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ActionSerde::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ActionSerde::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, InputId, OutputId, VJoyAxis};

    fn test_input_address() -> InputAddress {
        InputAddress::Bound {
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
    fn action_map_to_keyboard_behavior_roundtrips() {
        let action = Action::MapToKeyboard {
            key: KeyCombo {
                key: "Space".to_owned(),
                modifiers: vec![],
            },
            behavior: OutputBehavior::Pulse,
        };

        let json = serde_json::to_string(&action).unwrap();

        assert!(json.contains("\"type\":\"map_to_keyboard\""));
        assert!(json.contains("\"behavior\":\"pulse\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn old_keyboard_action_defaults_to_hold() {
        let json = r#"{"type":"map_to_keyboard","key":{"key":"A","modifiers":[]}}"#;

        let back: Action = serde_json::from_str(json).unwrap();

        assert_eq!(
            back,
            Action::MapToKeyboard {
                key: KeyCombo {
                    key: "A".to_owned(),
                    modifiers: vec![],
                },
                behavior: OutputBehavior::Hold,
            }
        );
    }

    #[test]
    fn action_map_to_mouse_button_pulse_roundtrips() {
        let action = Action::MapToMouse {
            target: MouseTarget::LeftButton,
            behavior: OutputBehavior::Pulse,
        };

        let json = serde_json::to_string(&action).unwrap();

        assert!(json.contains("\"type\":\"map_to_mouse\""));
        assert!(json.contains("\"target\":\"LeftButton\""));
        assert!(json.contains("\"behavior\":\"pulse\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn action_map_to_mouse_button_hold_roundtrips() {
        let action = Action::MapToMouse {
            target: MouseTarget::RightButton,
            behavior: OutputBehavior::Hold,
        };

        let json = serde_json::to_string(&action).unwrap();

        assert!(json.contains("\"type\":\"map_to_mouse\""));
        assert!(json.contains("\"target\":\"RightButton\""));
        assert!(json.contains("\"behavior\":\"hold\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn action_map_to_mouse_wheel_up_normalizes_hold_to_pulse() {
        let json = r#"{"type":"map_to_mouse","target":"WheelUp","behavior":"hold"}"#;

        let back: Action = serde_json::from_str(json).unwrap();
        let saved = serde_json::to_string(&back).unwrap();

        assert_eq!(
            back,
            Action::MapToMouse {
                target: MouseTarget::WheelUp,
                behavior: OutputBehavior::Pulse,
            }
        );
        assert!(saved.contains("\"behavior\":\"pulse\""));
    }

    #[test]
    fn action_map_to_mouse_wheel_down_normalizes_hold_to_pulse() {
        let json = r#"{"type":"map_to_mouse","target":"WheelDown","behavior":"hold"}"#;

        let back: Action = serde_json::from_str(json).unwrap();
        let saved = serde_json::to_string(&back).unwrap();

        assert_eq!(
            back,
            Action::MapToMouse {
                target: MouseTarget::WheelDown,
                behavior: OutputBehavior::Pulse,
            }
        );
        assert!(saved.contains("\"behavior\":\"pulse\""));
    }

    #[test]
    fn invalid_mouse_target_fails_to_load() {
        let json = r#"{"type":"map_to_mouse","target":"Sideways","behavior":"pulse"}"#;

        let err = serde_json::from_str::<Action>(json).unwrap_err();

        assert!(err.to_string().contains("unknown variant"));
    }

    #[test]
    fn invalid_output_behavior_fails_to_load() {
        let json = r#"{"type":"map_to_mouse","target":"LeftButton","behavior":"repeat"}"#;

        let err = serde_json::from_str::<Action>(json).unwrap_err();

        assert!(err.to_string().contains("unknown variant"));
    }

    #[test]
    fn mouse_target_labels_are_stable() {
        assert_eq!(MouseTarget::LeftButton.label(), "Left click");
        assert_eq!(MouseTarget::RightButton.label(), "Right click");
        assert_eq!(MouseTarget::MiddleButton.label(), "Middle click");
        assert_eq!(MouseTarget::BackButton.label(), "Back button");
        assert_eq!(MouseTarget::ForwardButton.label(), "Forward button");
        assert_eq!(MouseTarget::WheelUp.label(), "Wheel up");
        assert_eq!(MouseTarget::WheelDown.label(), "Wheel down");
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
