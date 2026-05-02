// Rust guideline compliant 2026-03-03

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};
use crate::types::{HatDirection, InputAddress};

/// Maximum nesting depth for condition trees.
const MAX_CONDITION_DEPTH: usize = 32;

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
    /// True when all inner conditions are true. An empty list is vacuously true.
    All {
        conditions: Vec<Condition>,
    },
    /// True when at least one inner condition is true. An empty list is vacuously false.
    Any {
        conditions: Vec<Condition>,
    },
    Not {
        condition: Box<Condition>,
    },
}

/// Validates that a condition tree does not exceed the maximum nesting depth.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] when the nesting depth exceeds
/// [`MAX_CONDITION_DEPTH`].
pub fn validate_depth(condition: &Condition, max_depth: usize) -> Result<()> {
    fn walk(condition: &Condition, remaining: usize) -> Result<()> {
        match condition {
            Condition::All { conditions } | Condition::Any { conditions } => {
                if remaining == 0 {
                    return Err(EngineError::InvalidConfig {
                        reason: format!(
                            "condition nesting exceeds maximum depth of {MAX_CONDITION_DEPTH}"
                        ),
                    });
                }
                for c in conditions {
                    walk(c, remaining - 1)?;
                }
            }
            Condition::Not { condition } => {
                if remaining == 0 {
                    return Err(EngineError::InvalidConfig {
                        reason: format!(
                            "condition nesting exceeds maximum depth of {MAX_CONDITION_DEPTH}"
                        ),
                    });
                }
                walk(condition, remaining - 1)?;
            }
            _ => {}
        }
        Ok(())
    }
    walk(condition, max_depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, InputId};

    fn test_input_address() -> InputAddress {
        InputAddress::Bound {
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
            input: InputAddress::Bound {
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
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Hat { index: 0 },
            },
            directions: vec![HatDirection::N, HatDirection::NE],
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

    // -- Depth validation -----------------------------------------------------

    #[test]
    fn validate_depth_flat_condition_passes() {
        let cond = Condition::ButtonPressed {
            input: test_input_address(),
        };
        validate_depth(&cond, MAX_CONDITION_DEPTH).unwrap();
    }

    #[test]
    fn validate_depth_nested_within_limit_passes() {
        // 3 levels deep: Not(All(ButtonPressed))
        let cond = Condition::Not {
            condition: Box::new(Condition::All {
                conditions: vec![Condition::ButtonPressed {
                    input: test_input_address(),
                }],
            }),
        };
        validate_depth(&cond, MAX_CONDITION_DEPTH).unwrap();
    }

    #[test]
    fn validate_depth_exceeds_limit_fails() {
        // Build a chain of Not() exceeding max_depth of 2
        let leaf = Condition::ButtonPressed {
            input: test_input_address(),
        };
        let depth_3 = Condition::Not {
            condition: Box::new(Condition::Not {
                condition: Box::new(Condition::Not {
                    condition: Box::new(leaf),
                }),
            }),
        };
        let result = validate_depth(&depth_3, 2);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("nesting exceeds maximum depth"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn validate_depth_exactly_at_limit_passes() {
        // depth 1: Not(ButtonPressed) with max_depth=1
        let cond = Condition::Not {
            condition: Box::new(Condition::ButtonPressed {
                input: test_input_address(),
            }),
        };
        validate_depth(&cond, 1).unwrap();
    }

    #[test]
    fn validate_depth_zero_limit_rejects_nested() {
        let cond = Condition::Not {
            condition: Box::new(Condition::ButtonPressed {
                input: test_input_address(),
            }),
        };
        assert!(
            validate_depth(&cond, 0).is_err(),
            "expected depth 0 to reject nested condition"
        );
    }

    #[test]
    fn validate_depth_zero_limit_accepts_leaf() {
        let cond = Condition::ButtonPressed {
            input: test_input_address(),
        };
        validate_depth(&cond, 0).unwrap();
    }

    #[test]
    fn validate_depth_all_with_wide_children_passes() {
        // All with 3 leaf children at depth 1
        let cond = Condition::All {
            conditions: vec![
                Condition::ButtonPressed {
                    input: test_input_address(),
                },
                Condition::ButtonReleased {
                    input: test_input_address(),
                },
                Condition::ButtonPressed {
                    input: test_input_address(),
                },
            ],
        };
        validate_depth(&cond, 1).unwrap();
    }
}
