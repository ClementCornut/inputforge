//! Scalar extraction and merge-polarity inference.
use inputforge_core::{
    pipeline::InputCache,
    state::AppState,
    types::{AxisPolarity, InputAddress, InputValue, MergeOp},
};
use inputforge_gui_dx::readouts::ChainStep;
/// Extract a scalar f64 from any `InputValue`.
pub(super) fn axis_f64(v: &InputValue) -> f64 {
    match v {
        InputValue::Axis { value, .. } => value.value(),
        InputValue::Button { pressed } => {
            if *pressed {
                1.0
            } else {
                0.0
            }
        }
        InputValue::Hat { .. } => 0.0,
    }
}

/// Infer the natural polarity of a merge result from the operator and
/// each input's polarity.
#[must_use]
pub(super) fn merge_output_polarity(
    op: MergeOp,
    primary: AxisPolarity,
    secondary: AxisPolarity,
) -> AxisPolarity {
    match op {
        MergeOp::Bidirectional => AxisPolarity::Bipolar,
        MergeOp::Average | MergeOp::Maximum => {
            if primary == secondary {
                primary
            } else {
                AxisPolarity::Bipolar
            }
        }
    }
}

/// Fold merge operations along one output path to infer terminal polarity.
#[must_use]
#[cfg(test)]
pub(super) fn infer_output_polarity(
    primary_polarity: AxisPolarity,
    merges_on_path: &[(MergeOp, AxisPolarity)],
) -> AxisPolarity {
    merges_on_path
        .iter()
        .fold(primary_polarity, |acc, (op, secondary)| {
            merge_output_polarity(*op, acc, *secondary)
        })
}

pub(super) fn compute_is_active(chain: &[ChainStep]) -> bool {
    chain.iter().all(|step| match step {
        ChainStep::Merge { .. } => true,
        ChainStep::Conditional { active, .. } => *active,
    })
}

pub(super) fn terminal_polarity(
    chain: &[ChainStep],
    primary: &InputAddress,
    state: &AppState,
) -> AxisPolarity {
    chain
        .iter()
        .rev()
        .find_map(|step| match step {
            ChainStep::Merge {
                polarity_at_step, ..
            } => Some(*polarity_at_step),
            ChainStep::Conditional { .. } => None,
        })
        .unwrap_or_else(|| state.input_cache.get_axis(primary).1)
}
