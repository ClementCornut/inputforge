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
}
