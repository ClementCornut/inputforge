// Rust guideline compliant 2026-05-02

use crate::action::Condition;
use crate::types::InputAddress;

use super::InputCache;

/// Evaluate a condition against the input cache.
///
/// Leaves whose `input` is [`InputAddress::Unbound`] return `false`: they are
/// incomplete predicates that the user has not yet bound to a real input.
/// `Not(Unbound)` therefore returns `true` (the natural consequence of
/// `!false`). The validator surfaces unbound leaves as malformed-hints
/// regardless, so users still get inline feedback that the condition is
/// incomplete.
#[must_use]
pub fn evaluate_condition(condition: &Condition, cache: &dyn InputCache) -> bool {
    match condition {
        Condition::ButtonPressed { input } => match input {
            InputAddress::Bound { .. } => cache.get_button(input),
            InputAddress::Unbound => false,
        },
        Condition::ButtonReleased { input } => match input {
            InputAddress::Bound { .. } => !cache.get_button(input),
            InputAddress::Unbound => false,
        },
        Condition::AxisInRange { input, min, max } => match input {
            InputAddress::Bound { .. } => {
                // Range thresholds are interpreted in the bipolar-encoded
                // [-1, 1] domain, regardless of the input's polarity. A
                // unipolar pedal at idle (encoded -1) compared against
                // `min: 0.0, max: 1.0` evaluates as out-of-range, which
                // matches existing F1-F9 condition semantics. Polarity is
                // intentionally unused.
                let (value, _polarity) = cache.get_axis(input);
                value >= *min && value <= *max
            }
            InputAddress::Unbound => false,
        },
        Condition::HatDirection { input, directions } => match input {
            InputAddress::Bound { .. } => {
                let current = cache.get_hat(input);
                // O(n) linear scan is acceptable: HatDirection has at most 9 variants.
                directions.contains(&current)
            }
            InputAddress::Unbound => false,
        },
        Condition::All { conditions } => conditions.iter().all(|c| evaluate_condition(c, cache)),
        Condition::Any { conditions } => conditions.iter().any(|c| evaluate_condition(c, cache)),
        Condition::Not { condition } => !evaluate_condition(condition, cache),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{MockCache, axis_input_address, button_input_address};
    use super::*;
    use crate::types::{DeviceId, HatDirection, InputId};

    // -- Nested conditions ----------------------------------------------------

    #[test]
    fn nested_condition_all() {
        let mut cache = MockCache::new();
        let btn_a = button_input_address();
        let btn_b = InputAddress::Bound {
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

        let btn_b = InputAddress::Bound {
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
        InputAddress::Bound {
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
        let btn_b = InputAddress::Bound {
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

    // -- Unbound leaves -------------------------------------------------------

    #[test]
    fn button_pressed_unbound_is_false() {
        let cache = MockCache::new();
        let cond = Condition::ButtonPressed {
            input: InputAddress::Unbound,
        };
        assert!(!evaluate_condition(&cond, &cache));
    }

    #[test]
    fn button_released_unbound_is_false() {
        // Released-on-Unbound is also false: the user has not picked a binding,
        // so the condition is incomplete and must not satisfy.
        let cache = MockCache::new();
        let cond = Condition::ButtonReleased {
            input: InputAddress::Unbound,
        };
        assert!(!evaluate_condition(&cond, &cache));
    }

    #[test]
    fn axis_in_range_unbound_is_false() {
        let cache = MockCache::new();
        let cond = Condition::AxisInRange {
            input: InputAddress::Unbound,
            min: -1.0,
            max: 1.0,
        };
        assert!(!evaluate_condition(&cond, &cache));
    }

    #[test]
    fn hat_direction_unbound_is_false() {
        let cache = MockCache::new();
        let cond = Condition::HatDirection {
            input: InputAddress::Unbound,
            directions: vec![HatDirection::N],
        };
        assert!(!evaluate_condition(&cond, &cache));
    }

    #[test]
    fn not_unbound_is_true() {
        // !false == true is the natural consequence of the leaf-Unbound semantics.
        // This is documented behaviour, not a bug; the validator (Task 9) flags
        // unbound leaves as malformed-hints regardless.
        let cache = MockCache::new();
        let cond = Condition::Not {
            condition: Box::new(Condition::ButtonPressed {
                input: InputAddress::Unbound,
            }),
        };
        assert!(evaluate_condition(&cond, &cache));
    }

    #[test]
    fn all_with_unbound_leaf_is_false() {
        // An `All` combinator must collapse to `false` when any leaf has an
        // Unbound input, even if the other leaves are satisfied. The leaf-
        // Unbound short-circuit propagates through `All` via the existing
        // `iter().all(...)` semantics; this test locks in that propagation.
        let mut cache = MockCache::new();
        let btn = button_input_address();
        cache.buttons.insert(btn.clone(), true);
        let cond = Condition::All {
            conditions: vec![
                Condition::ButtonPressed { input: btn },
                Condition::ButtonPressed {
                    input: InputAddress::Unbound,
                },
            ],
        };
        assert!(!evaluate_condition(&cond, &cache));
    }

    #[test]
    fn any_with_unbound_leaf_falls_through_to_bound() {
        // An `Any` combinator with an Unbound leaf falls through to the bound
        // leaves; if any of those is satisfied, the result is `true`. The
        // Unbound leaf neither short-circuits nor poisons the disjunction.
        let mut cache = MockCache::new();
        let btn = button_input_address();
        cache.buttons.insert(btn.clone(), true);
        let cond = Condition::Any {
            conditions: vec![
                Condition::ButtonPressed {
                    input: InputAddress::Unbound,
                },
                Condition::ButtonPressed { input: btn },
            ],
        };
        assert!(evaluate_condition(&cond, &cache));
    }
}
