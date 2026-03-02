// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use crate::types::{HatDirection, InputAddress};

/// A condition that can be evaluated against the current input state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    ButtonPressed {
        input: InputAddress,
    },
    ButtonReleased {
        input: InputAddress,
    },
    AxisInRange {
        input: InputAddress,
        min: f64,
        max: f64,
    },
    /// True when the hat at `input` is pointing in any of `directions`.
    HatDirection {
        input: InputAddress,
        directions: Vec<HatDirection>,
    },
    All {
        conditions: Vec<Condition>,
    },
    Any {
        conditions: Vec<Condition>,
    },
    Not {
        condition: Box<Condition>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, InputId};

    fn test_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        }
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
    fn condition_hat_direction_serde_roundtrip() {
        let cond = Condition::HatDirection {
            input: InputAddress {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Hat { index: 0 },
            },
            directions: vec![
                crate::types::HatDirection::N,
                crate::types::HatDirection::NE,
            ],
        };
        let json = serde_json::to_string(&cond).unwrap();
        assert!(json.contains("\"type\":\"hat_direction\""));
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
}
