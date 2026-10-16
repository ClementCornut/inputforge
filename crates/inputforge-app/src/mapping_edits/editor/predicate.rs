//! Predicate field changes preserve existing operands and reject malformed raw numbers.
use inputforge_core::{action::Condition, types::InputAddress};
use inputforge_gui_dx::mapping_edits::Field;
pub(super) fn replace(current: &Condition, field: &Field) -> Option<Condition> {
    let mut condition = current.clone();
    match (&mut condition, field) {
        (_, Field::ConditionKind(kind)) => {
            let current_kind = match current {
                Condition::ButtonPressed { .. } => "ButtonPressed",
                Condition::ButtonReleased { .. } => "ButtonReleased",
                Condition::AxisInRange { .. } => "AxisInRange",
                Condition::HatDirection { .. } => "HatDirection",
                Condition::All { .. } => "All",
                Condition::Any { .. } => "Any",
                Condition::Not { .. } => "Not",
            };
            if kind == current_kind {
                return Some(current.clone());
            }
            let input = match current {
                Condition::ButtonPressed { input }
                | Condition::ButtonReleased { input }
                | Condition::AxisInRange { input, .. }
                | Condition::HatDirection { input, .. } => input.clone(),
                _ => InputAddress::Unbound,
            };
            condition = match kind.as_str() {
                "ButtonPressed" => Condition::ButtonPressed { input },
                "ButtonReleased" => Condition::ButtonReleased { input },
                "AxisInRange" => Condition::AxisInRange {
                    input,
                    min: -1.0,
                    max: 1.0,
                },
                "HatDirection" => Condition::HatDirection {
                    input,
                    directions: vec![],
                },
                "All" => Condition::All { conditions: vec![] },
                "Any" => Condition::Any { conditions: vec![] },
                "Not" => Condition::Not {
                    condition: Box::new(Condition::ButtonPressed { input }),
                },
                _ => return None,
            };
        }
        (
            Condition::ButtonPressed { input }
            | Condition::ButtonReleased { input }
            | Condition::AxisInRange { input, .. }
            | Condition::HatDirection { input, .. },
            Field::ConditionInput(value),
        ) => input.clone_from(value),
        (Condition::AxisInRange { min, .. }, Field::ConditionMin(raw)) => *min = number(raw)?,
        (Condition::AxisInRange { max, .. }, Field::ConditionMax(raw)) => *max = number(raw)?,
        (Condition::HatDirection { directions, .. }, Field::ConditionDirection(value)) => {
            if directions.contains(value) {
                directions.retain(|d| d != value);
            } else {
                directions.push(*value);
            }
        }
        _ => return None,
    }
    Some(condition)
}
pub(crate) fn number(raw: &str) -> Option<f64> {
    let value = raw.parse::<f64>().ok()?;
    (value.is_finite() && (-1.0..=1.0).contains(&value)).then_some(value)
}
