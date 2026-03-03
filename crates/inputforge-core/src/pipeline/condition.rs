// Rust guideline compliant 2026-03-03

use crate::action::Condition;

use super::InputCache;

/// Evaluate a condition against the input cache.
#[must_use]
pub fn evaluate_condition(condition: &Condition, cache: &dyn InputCache) -> bool {
    match condition {
        Condition::ButtonPressed { input } => cache.get_button(input),
        Condition::ButtonReleased { input } => !cache.get_button(input),
        Condition::AxisInRange { input, min, max } => {
            let value = cache.get_axis(input);
            value >= *min && value <= *max
        }
        Condition::HatDirection { input, directions } => {
            let current = cache.get_hat(input);
            // O(n) linear scan is acceptable: HatDirection has at most 9 variants.
            directions.contains(&current)
        }
        Condition::All { conditions } => conditions.iter().all(|c| evaluate_condition(c, cache)),
        Condition::Any { conditions } => conditions.iter().any(|c| evaluate_condition(c, cache)),
        Condition::Not { condition } => !evaluate_condition(condition, cache),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{MockCache, axis_input_address, button_input_address};
    use super::*;
    use crate::types::{DeviceId, HatDirection, InputAddress, InputId};

    // -- Nested conditions ----------------------------------------------------

    #[test]
    fn nested_condition_all() {
        let mut cache = MockCache::new();
        let btn_a = button_input_address();
        let btn_b = InputAddress {
            device: DeviceId("stick-1".to_owned()),
            input: InputId::Button { index: 1 },
        };
        cache.buttons.insert(btn_a.clone(), true);
        cache.buttons.insert(btn_b.clone(), true);

        let condition = Condition::All {
            conditions: vec![
                Condition::ButtonPressed { input: btn_a },
                Condition::ButtonPressed { input: btn_b },
            ],
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    #[test]
    fn nested_condition_any() {
        let mut cache = MockCache::new();
        let btn_a = button_input_address();
        cache.buttons.insert(btn_a.clone(), false);

        let btn_b = InputAddress {
            device: DeviceId("stick-1".to_owned()),
            input: InputId::Button { index: 1 },
        };
        cache.buttons.insert(btn_b.clone(), true);

        let condition = Condition::Any {
            conditions: vec![
                Condition::ButtonPressed { input: btn_a },
                Condition::ButtonPressed { input: btn_b },
            ],
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    #[test]
    fn nested_condition_not() {
        let cache = MockCache::new(); // button defaults to false
        let condition = Condition::Not {
            condition: Box::new(Condition::ButtonPressed {
                input: button_input_address(),
            }),
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    // -- ButtonReleased -------------------------------------------------------

    #[test]
    fn button_released_true_when_not_pressed() {
        let cache = MockCache::new(); // button defaults to false
        let condition = Condition::ButtonReleased {
            input: button_input_address(),
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    #[test]
    fn button_released_false_when_pressed() {
        let mut cache = MockCache::new();
        cache.buttons.insert(button_input_address(), true);
        let condition = Condition::ButtonReleased {
            input: button_input_address(),
        };
        assert!(!evaluate_condition(&condition, &cache));
    }

    // -- AxisInRange ----------------------------------------------------------

    #[test]
    fn axis_in_range_true() {
        let mut cache = MockCache::new();
        let addr = axis_input_address();
        cache.axes.insert(addr.clone(), 0.5);
        let condition = Condition::AxisInRange {
            input: addr,
            min: 0.0,
            max: 1.0,
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    #[test]
    fn axis_in_range_false() {
        let mut cache = MockCache::new();
        let addr = axis_input_address();
        cache.axes.insert(addr.clone(), 0.5);
        let condition = Condition::AxisInRange {
            input: addr,
            min: 0.6,
            max: 1.0,
        };
        assert!(!evaluate_condition(&condition, &cache));
    }

    // -- HatDirection ---------------------------------------------------------

    fn hat_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("stick-1".to_owned()),
            input: InputId::Hat { index: 0 },
        }
    }

    #[test]
    fn hat_direction_matches_single() {
        let mut cache = MockCache::new();
        let addr = hat_input_address();
        cache.hats.insert(addr.clone(), HatDirection::N);

        let condition = Condition::HatDirection {
            input: addr,
            directions: vec![HatDirection::N],
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    #[test]
    fn hat_direction_matches_any_of_multiple() {
        let mut cache = MockCache::new();
        let addr = hat_input_address();
        cache.hats.insert(addr.clone(), HatDirection::NE);

        let condition = Condition::HatDirection {
            input: addr,
            directions: vec![HatDirection::N, HatDirection::NE, HatDirection::NW],
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    #[test]
    fn hat_direction_no_match() {
        let mut cache = MockCache::new();
        let addr = hat_input_address();
        cache.hats.insert(addr.clone(), HatDirection::S);

        let condition = Condition::HatDirection {
            input: addr,
            directions: vec![HatDirection::N, HatDirection::NE],
        };
        assert!(!evaluate_condition(&condition, &cache));
    }

    #[test]
    fn hat_direction_defaults_to_center() {
        let cache = MockCache::new(); // hat defaults to Center
        let condition = Condition::HatDirection {
            input: hat_input_address(),
            directions: vec![HatDirection::Center],
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    #[test]
    fn hat_direction_empty_directions_never_matches() {
        let cache = MockCache::new(); // hat defaults to Center
        let condition = Condition::HatDirection {
            input: hat_input_address(),
            directions: vec![],
        };
        assert!(!evaluate_condition(&condition, &cache));
    }

    // -- Deeply nested conditions ---------------------------------------------

    #[test]
    fn deeply_nested_all_any_not() {
        let mut cache = MockCache::new();
        let btn_a = button_input_address();
        let btn_b = InputAddress {
            device: DeviceId("stick-1".to_owned()),
            input: InputId::Button { index: 1 },
        };
        let axis = axis_input_address();

        cache.buttons.insert(btn_a.clone(), true);
        cache.buttons.insert(btn_b.clone(), false);
        cache.axes.insert(axis.clone(), 0.5);

        // All(
        //   ButtonPressed(btn_a),       -> true
        //   Any(
        //     ButtonPressed(btn_b),      -> false
        //     AxisInRange(axis, 0..1),   -> true  => Any -> true
        //   ),
        //   Not(ButtonPressed(btn_b)),   -> true
        // ) => All -> true
        let condition = Condition::All {
            conditions: vec![
                Condition::ButtonPressed {
                    input: btn_a.clone(),
                },
                Condition::Any {
                    conditions: vec![
                        Condition::ButtonPressed {
                            input: btn_b.clone(),
                        },
                        Condition::AxisInRange {
                            input: axis,
                            min: 0.0,
                            max: 1.0,
                        },
                    ],
                },
                Condition::Not {
                    condition: Box::new(Condition::ButtonPressed { input: btn_b }),
                },
            ],
        };
        assert!(evaluate_condition(&condition, &cache));
    }

    #[test]
    fn deeply_nested_fails_when_inner_not_inverts() {
        let mut cache = MockCache::new();
        let btn = button_input_address();
        cache.buttons.insert(btn.clone(), true);

        // Not(Not(Not(ButtonPressed(btn))))
        // = Not(Not(false)) = Not(true) = false
        let condition = Condition::Not {
            condition: Box::new(Condition::Not {
                condition: Box::new(Condition::Not {
                    condition: Box::new(Condition::ButtonPressed { input: btn }),
                }),
            }),
        };
        assert!(!evaluate_condition(&condition, &cache));
    }
}
