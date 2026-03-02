// Rust guideline compliant 2026-03-02

use crate::types::MergeOp;

/// Merge two axis values using the specified operation.
#[must_use]
pub fn merge_axes(first: f64, second: f64, operation: MergeOp) -> f64 {
    match operation {
        MergeOp::Bidirectional => (first - second).clamp(-1.0, 1.0),
        MergeOp::Average => f64::midpoint(first, second),
        MergeOp::Maximum => {
            if first.abs() >= second.abs() {
                first
            } else {
                second
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    #[test]
    fn bidirectional_subtracts_and_clamps() {
        assert!((merge_axes(0.8, 0.3, MergeOp::Bidirectional) - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn average_computes_midpoint() {
        assert!((merge_axes(0.8, 0.4, MergeOp::Average) - 0.6).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_picks_larger_absolute() {
        // |-0.9| > |0.3|, so the result is -0.9
        assert!((merge_axes(0.3, -0.9, MergeOp::Maximum) - (-0.9)).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_first_when_larger_absolute() {
        // |-0.8| > |0.3|, so the result is -0.8 (first wins)
        assert!((merge_axes(-0.8, 0.3, MergeOp::Maximum) - (-0.8)).abs() < TOLERANCE);
    }

    // -- Pedal integration tests: bidirectional merge -------------------------
    //
    // Rudder pedals scenario: left pedal controls one axis, right pedal another.
    // Bidirectional merge combines them: result = (left - right), clamped to [-1, 1].
    // Convention: left pedal produces negative values, right pedal positive.

    #[test]
    fn pedal_both_fully_depressed_cancel_out() {
        // Both pedals fully depressed: left=-1, right=-1
        // Bidirectional: (-1) - (-1) = 0
        assert!(merge_axes(-1.0, -1.0, MergeOp::Bidirectional).abs() < TOLERANCE);
    }

    #[test]
    fn pedal_left_full_right_released() {
        // Left pedal full: -1, right pedal released: 0
        // Bidirectional: (-1) - 0 = -1 (full left rudder)
        assert!((merge_axes(-1.0, 0.0, MergeOp::Bidirectional) - (-1.0)).abs() < TOLERANCE);
    }

    #[test]
    fn pedal_right_full_left_released() {
        // Left pedal released: 0, right pedal full: -1
        // Bidirectional: 0 - (-1) = 1 (full right rudder)
        assert!((merge_axes(0.0, -1.0, MergeOp::Bidirectional) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn pedal_both_centered() {
        // Both at rest: 0, 0
        assert!(merge_axes(0.0, 0.0, MergeOp::Bidirectional).abs() < TOLERANCE);
    }

    #[test]
    fn pedal_partial_inputs() {
        // Left half depressed: -0.5, right quarter: -0.25
        // Bidirectional: (-0.5) - (-0.25) = -0.25
        assert!((merge_axes(-0.5, -0.25, MergeOp::Bidirectional) - (-0.25)).abs() < TOLERANCE);
    }

    #[test]
    fn pedal_extreme_clamps() {
        // Both at opposite extremes: left=1, right=-1
        // Bidirectional: 1 - (-1) = 2, clamped to 1.0
        assert!((merge_axes(1.0, -1.0, MergeOp::Bidirectional) - 1.0).abs() < TOLERANCE);
    }
}
