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
