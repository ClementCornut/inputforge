//! Predicate leaf evaluation and de-duplication.
use super::analyze::AnalysisContext;
use inputforge_core::{
    action::Condition,
    pipeline::InputCache,
    types::{HatDirection, InputAddress},
};
use inputforge_gui_dx::readouts::{LiveReadoutModel, PredicateDescriptor, PredicateKind};
use std::collections::HashSet;
/// Per-leaf state snapshot used by IN-block predicate chip rendering.
pub(super) fn evaluate_leaf_state(
    input: &InputAddress,
    kind: &PredicateKind,
    cache: &dyn InputCache,
) -> bool {
    use inputforge_core::pipeline::evaluate_condition;

    let synthetic = match kind {
        PredicateKind::ButtonPressed => Condition::ButtonPressed {
            input: input.clone(),
        },
        PredicateKind::ButtonReleased => Condition::ButtonReleased {
            input: input.clone(),
        },
        PredicateKind::AxisInRange { min, max } => Condition::AxisInRange {
            input: input.clone(),
            min: *min,
            max: *max,
        },
        PredicateKind::HatDirection { directions } => Condition::HatDirection {
            input: input.clone(),
            directions: directions.clone(),
        },
    };
    evaluate_condition(&synthetic, cache)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct PredicateDedupKey {
    input: InputAddress,
    kind: PredicateKindDedupKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PredicateKindDedupKey {
    ButtonPressed,
    ButtonReleased,
    AxisInRange { min_bits: u64, max_bits: u64 },
    HatDirection { directions: Vec<u8> },
}

impl PredicateDedupKey {
    fn new(input: &InputAddress, kind: &PredicateKind) -> Self {
        Self {
            input: input.clone(),
            kind: PredicateKindDedupKey::from(kind),
        }
    }
}

impl From<&PredicateKind> for PredicateKindDedupKey {
    fn from(kind: &PredicateKind) -> Self {
        match kind {
            PredicateKind::ButtonPressed => Self::ButtonPressed,
            PredicateKind::ButtonReleased => Self::ButtonReleased,
            PredicateKind::AxisInRange { min, max } => Self::AxisInRange {
                min_bits: normalized_float_bits(*min),
                max_bits: normalized_float_bits(*max),
            },
            PredicateKind::HatDirection { directions } => Self::HatDirection {
                directions: canonical_hat_direction_keys(directions),
            },
        }
    }
}

fn normalized_float_bits(value: f64) -> u64 {
    if value == 0.0 {
        0.0f64.to_bits()
    } else {
        value.to_bits()
    }
}

fn canonical_hat_direction_keys(directions: &[HatDirection]) -> Vec<u8> {
    let mut keys: Vec<_> = directions.iter().copied().map(hat_direction_key).collect();
    keys.sort_unstable();
    keys.dedup();
    keys
}

fn hat_direction_key(direction: HatDirection) -> u8 {
    match direction {
        HatDirection::Center => 0,
        HatDirection::N => 1,
        HatDirection::NE => 2,
        HatDirection::E => 3,
        HatDirection::SE => 4,
        HatDirection::S => 5,
        HatDirection::SW => 6,
        HatDirection::W => 7,
        HatDirection::NW => 8,
    }
}

pub(super) fn collect_condition_predicates(
    context: &AnalysisContext<'_>,
    condition: &Condition,
    model: &mut LiveReadoutModel,
    predicate_keys: &mut HashSet<PredicateDedupKey>,
) {
    match condition {
        Condition::ButtonPressed { input } => emit_predicate(
            context,
            model,
            predicate_keys,
            input,
            PredicateKind::ButtonPressed,
        ),
        Condition::ButtonReleased { input } => emit_predicate(
            context,
            model,
            predicate_keys,
            input,
            PredicateKind::ButtonReleased,
        ),
        Condition::AxisInRange { input, min, max } => emit_predicate(
            context,
            model,
            predicate_keys,
            input,
            PredicateKind::AxisInRange {
                min: *min,
                max: *max,
            },
        ),
        Condition::HatDirection { input, directions } => emit_predicate(
            context,
            model,
            predicate_keys,
            input,
            PredicateKind::HatDirection {
                directions: directions.clone(),
            },
        ),
        Condition::All { conditions } | Condition::Any { conditions } => {
            for condition in conditions {
                collect_condition_predicates(context, condition, model, predicate_keys);
            }
        }
        Condition::Not { condition } => {
            collect_condition_predicates(context, condition, model, predicate_keys);
        }
    }
}

fn emit_predicate(
    context: &AnalysisContext<'_>,
    model: &mut LiveReadoutModel,
    predicate_keys: &mut HashSet<PredicateDedupKey>,
    input: &InputAddress,
    kind: PredicateKind,
) {
    if !predicate_keys.insert(PredicateDedupKey::new(input, &kind)) {
        return;
    }

    model.predicates.push(PredicateDescriptor {
        inputs: vec![input.clone()],
        state: evaluate_leaf_state(input, &kind, &context.state.input_cache),
        label: inputforge_gui_dx::readouts::labels::format_predicate_chip_label(input, context.cfg),
        kind,
    });
}
