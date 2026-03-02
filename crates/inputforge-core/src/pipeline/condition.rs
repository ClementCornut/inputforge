// Rust guideline compliant 2026-03-02

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
        Condition::All { conditions } => conditions.iter().all(|c| evaluate_condition(c, cache)),
        Condition::Any { conditions } => conditions.iter().any(|c| evaluate_condition(c, cache)),
        Condition::Not { condition } => !evaluate_condition(condition, cache),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::types::{DeviceId, InputAddress, InputId};

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
    }

    fn button_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("stick-1".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

    fn axis_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("stick-1".to_owned()),
            input: InputId::Axis { index: 0 },
        }
    }

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
}
