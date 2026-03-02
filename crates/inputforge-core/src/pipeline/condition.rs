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
