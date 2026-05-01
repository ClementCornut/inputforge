// Rust guideline compliant 2026-05-01

use crate::processing::into_natural_domain;
use crate::types::{AxisPolarity, MergeOp};

/// Merge two axis values using the specified operation.
///
/// Inputs are bipolar-encoded `[-1, 1]` regardless of natural polarity.
/// `Bidirectional` and `Maximum` consume each input's polarity to do
/// their math in the natural domain, then return a bipolar-encoded
/// result. `Average` is self-correcting under the encoded->natural
/// remap the GUI applies downstream and ignores polarity.
///
/// The return value stays bipolar-encoded so downstream pipeline
/// actions (curves, deadzone, `MapToVJoy`) continue to operate without
/// polarity awareness.
#[must_use]
pub fn merge_axes(
    first: f64,
    second: f64,
    operation: MergeOp,
    first_polarity: AxisPolarity,
    second_polarity: AxisPolarity,
) -> f64 {
    match operation {
        MergeOp::Bidirectional => {
            // Subtract in natural domain. Encoded subtraction over
            // unipolar inputs scales by 2x (encoded = 2*natural - 1, so
            // diff_encoded = 2*diff_natural), which spuriously maxes out
            // the bar when one pedal is half-pressed and the other idle.
            // Pre-remap each input to its natural domain so the diff
            // matches user intent.
            let first_natural = into_natural_domain(first, first_polarity);
            let second_natural = into_natural_domain(second, second_polarity);
            (first_natural - second_natural).clamp(-1.0, 1.0)
        }
        MergeOp::Average => f64::midpoint(first, second).clamp(-1.0, 1.0),
        MergeOp::Maximum => {
            // Compare in natural domain so a half-pressed unipolar pedal
            // (encoded 0, natural 0.5) beats an idle unipolar pedal
            // (encoded -1, natural 0). Return the winner's encoded value
            // so the pipeline downstream still sees [-1, 1].
            let first_natural = into_natural_domain(first, first_polarity);
            let second_natural = into_natural_domain(second, second_polarity);
            let winner = if first_natural.abs() >= second_natural.abs() {
                first
            } else {
                second
            };
            winner.clamp(-1.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    /// Default polarities (`Bipolar, Bipolar`) used by tests that
    /// pre-date polarity-awareness. Keeps existing `Maximum` math
    /// (encoded-domain abs comparison) unchanged.
    const BB: (AxisPolarity, AxisPolarity) = (AxisPolarity::Bipolar, AxisPolarity::Bipolar);

    #[test]
    fn bidirectional_subtracts_and_clamps() {
        assert!((merge_axes(0.8, 0.3, MergeOp::Bidirectional, BB.0, BB.1) - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn average_computes_midpoint() {
        assert!((merge_axes(0.8, 0.4, MergeOp::Average, BB.0, BB.1) - 0.6).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_picks_larger_absolute() {
        // |-0.9| > |0.3|, both bipolar, so the result is -0.9.
        assert!((merge_axes(0.3, -0.9, MergeOp::Maximum, BB.0, BB.1) - (-0.9)).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_first_when_larger_absolute() {
        // |-0.8| > |0.3|, both bipolar, so the result is -0.8 (first wins).
        assert!((merge_axes(-0.8, 0.3, MergeOp::Maximum, BB.0, BB.1) - (-0.8)).abs() < TOLERANCE);
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
        assert!(merge_axes(-1.0, -1.0, MergeOp::Bidirectional, BB.0, BB.1).abs() < TOLERANCE);
    }

    #[test]
    fn pedal_left_full_right_released() {
        // Left pedal full: -1, right pedal released: 0
        // Bidirectional: (-1) - 0 = -1 (full left rudder)
        assert!(
            (merge_axes(-1.0, 0.0, MergeOp::Bidirectional, BB.0, BB.1) - (-1.0)).abs() < TOLERANCE
        );
    }

    #[test]
    fn pedal_right_full_left_released() {
        // Left pedal released: 0, right pedal full: -1
        // Bidirectional: 0 - (-1) = 1 (full right rudder)
        assert!(
            (merge_axes(0.0, -1.0, MergeOp::Bidirectional, BB.0, BB.1) - 1.0).abs() < TOLERANCE
        );
    }

    #[test]
    fn pedal_both_centered() {
        // Both at rest: 0, 0
        assert!(merge_axes(0.0, 0.0, MergeOp::Bidirectional, BB.0, BB.1).abs() < TOLERANCE);
    }

    #[test]
    fn pedal_partial_inputs() {
        // Left half depressed: -0.5, right quarter: -0.25
        // Bidirectional: (-0.5) - (-0.25) = -0.25
        assert!(
            (merge_axes(-0.5, -0.25, MergeOp::Bidirectional, BB.0, BB.1) - (-0.25)).abs()
                < TOLERANCE
        );
    }

    #[test]
    fn pedal_extreme_clamps() {
        // Both at opposite extremes: left=1, right=-1
        // Bidirectional: 1 - (-1) = 2, clamped to 1.0
        assert!(
            (merge_axes(1.0, -1.0, MergeOp::Bidirectional, BB.0, BB.1) - 1.0).abs() < TOLERANCE
        );
    }

    // -- Out-of-range input clamping ------------------------------------------

    #[test]
    fn average_out_of_range_clamps_positive() {
        // midpoint(1.5, 1.5) = 1.5, clamped to 1.0
        assert!((merge_axes(1.5, 1.5, MergeOp::Average, BB.0, BB.1) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn average_out_of_range_clamps_negative() {
        // midpoint(-2.0, -1.0) = -1.5, clamped to -1.0
        assert!((merge_axes(-2.0, -1.0, MergeOp::Average, BB.0, BB.1) - (-1.0)).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_out_of_range_clamps_positive() {
        // |2.0| > |0.5|, result = 2.0, clamped to 1.0
        assert!((merge_axes(2.0, 0.5, MergeOp::Maximum, BB.0, BB.1) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_out_of_range_clamps_negative() {
        // |0.3| < |-1.5|, result = -1.5, clamped to -1.0
        assert!((merge_axes(0.3, -1.5, MergeOp::Maximum, BB.0, BB.1) - (-1.0)).abs() < TOLERANCE);
    }

    // -- Maximum: unipolar polarity awareness (the headline fix) --------------
    //
    // Unipolar pedals are bipolar-encoded with -1 = idle, +1 = full.
    // The pre-Task-2 implementation compared encoded magnitudes, so an
    // idle pedal (|encoded -1| = 1) beat a half-pressed pedal
    // (|encoded 0| = 0). Task 2 compares natural-domain magnitudes
    // instead so "more pressed wins".

    const UU: (AxisPolarity, AxisPolarity) = (AxisPolarity::Unipolar, AxisPolarity::Unipolar);

    #[test]
    fn maximum_unipolar_half_press_beats_idle() {
        // First pedal half-pressed (encoded 0, natural 0.5), second idle
        // (encoded -1, natural 0). First wins; return its encoded value.
        assert!((merge_axes(0.0, -1.0, MergeOp::Maximum, UU.0, UU.1) - 0.0).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_unipolar_idle_beats_idle_first_wins_tiebreak() {
        // Both idle, natural 0 each. Tied: first wins.
        assert!((merge_axes(-1.0, -1.0, MergeOp::Maximum, UU.0, UU.1) - (-1.0)).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_unipolar_full_press_beats_half_press() {
        // First fully pressed (encoded 1, natural 1), second half
        // (encoded 0, natural 0.5). First wins.
        assert!((merge_axes(1.0, 0.0, MergeOp::Maximum, UU.0, UU.1) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_unipolar_second_pressed_beats_first_idle() {
        // First idle, second half-pressed. Second wins.
        assert!((merge_axes(-1.0, 0.0, MergeOp::Maximum, UU.0, UU.1) - 0.0).abs() < TOLERANCE);
    }

    #[test]
    fn maximum_unipolar_swapped_order_picks_more_pressed_pedal() {
        // Same as `half_press_beats_idle` but with the inputs swapped.
        // Comparison happens in natural domain so the swap should still
        // pick the half-pressed pedal.
        assert!((merge_axes(-1.0, 0.5, MergeOp::Maximum, UU.0, UU.1) - 0.5).abs() < TOLERANCE);
    }

    // -- Maximum: mixed polarity ----------------------------------------------

    #[test]
    fn maximum_mixed_unipolar_pressed_beats_bipolar_near_center() {
        // Unipolar half-press (natural 0.5) vs bipolar slight-positive
        // (natural 0.1). Unipolar wins; returns its encoded value (0).
        assert!(
            (merge_axes(
                0.0,
                0.1,
                MergeOp::Maximum,
                AxisPolarity::Unipolar,
                AxisPolarity::Bipolar
            ) - 0.0)
                .abs()
                < TOLERANCE
        );
    }

    #[test]
    fn maximum_mixed_bipolar_extreme_beats_unipolar_idle() {
        // Bipolar full-negative (natural -1, abs 1) vs unipolar idle
        // (natural 0, abs 0). Bipolar wins.
        assert!(
            (merge_axes(
                -1.0,
                -1.0,
                MergeOp::Maximum,
                AxisPolarity::Bipolar,
                AxisPolarity::Unipolar
            ) - (-1.0))
                .abs()
                < TOLERANCE
        );
    }

    // -- Bidirectional: natural-domain subtraction ---------------------------

    #[test]
    fn bidirectional_unipolar_pair_at_idle_centers() {
        // Two unipolar pedals at idle (encoded -1, -1; natural 0, 0):
        // diff = 0. Bipolar-encoded center, natural rudder rest.
        assert!(merge_axes(-1.0, -1.0, MergeOp::Bidirectional, UU.0, UU.1).abs() < TOLERANCE);
    }

    #[test]
    fn bidirectional_unipolar_one_pedal_half_pressed_gives_half_deflection() {
        // The user-reported bug: half-press one pedal, idle the other.
        // Encoded (0, -1); natural (0.5, 0); diff = 0.5. Pre-fix the
        // encoded math returned 1.0 (full deflection) for half-press.
        assert!(
            (merge_axes(0.0, -1.0, MergeOp::Bidirectional, UU.0, UU.1) - 0.5).abs() < TOLERANCE
        );
    }

    #[test]
    fn bidirectional_unipolar_one_pedal_full_press_gives_full_deflection() {
        // Encoded (1, -1); natural (1, 0); diff = 1. Same as the
        // pre-fix encoded result thanks to the clamp; this test pins
        // the extreme so a future regression is caught.
        assert!(
            (merge_axes(1.0, -1.0, MergeOp::Bidirectional, UU.0, UU.1) - 1.0).abs() < TOLERANCE
        );
    }

    #[test]
    fn bidirectional_unipolar_both_half_pressed_centers() {
        // Encoded (0, 0); natural (0.5, 0.5); diff = 0. Both pedals
        // pressed equally produces the rudder-center value, regardless
        // of how far down they are.
        assert!(merge_axes(0.0, 0.0, MergeOp::Bidirectional, UU.0, UU.1).abs() < TOLERANCE);
    }

    #[test]
    fn bidirectional_natural_domain_for_mixed_polarity() {
        // The pre-fix `bidirectional_unchanged_for_mixed_polarity` test
        // asserted that all four polarity combos of (0.5, 0.2) yielded
        // the same encoded result (0.3). That was the bug: encoded
        // subtraction over unipolar inputs scales by 2x, so polarity
        // must change the result. Pin the correct natural-domain
        // behavior across all four combos.
        //
        // Inputs: encoded (0.5, 0.2). Natural map per polarity:
        //   Bipolar:  natural == encoded.
        //   Unipolar: natural = (encoded + 1) / 2 = midpoint(encoded, 1).
        let bb = merge_axes(0.5, 0.2, MergeOp::Bidirectional, BB.0, BB.1);
        let ub = merge_axes(
            0.5,
            0.2,
            MergeOp::Bidirectional,
            AxisPolarity::Unipolar,
            AxisPolarity::Bipolar,
        );
        let bu = merge_axes(
            0.5,
            0.2,
            MergeOp::Bidirectional,
            AxisPolarity::Bipolar,
            AxisPolarity::Unipolar,
        );
        let uu = merge_axes(0.5, 0.2, MergeOp::Bidirectional, UU.0, UU.1);
        // BB: 0.5 - 0.2 = 0.3.
        assert!((bb - 0.3).abs() < TOLERANCE, "bb expected 0.3, got {bb}");
        // UB: natural(0.5) Unipolar = 0.75, natural(0.2) Bipolar = 0.2;
        // diff = 0.55.
        assert!((ub - 0.55).abs() < TOLERANCE, "ub expected 0.55, got {ub}");
        // BU: natural(0.5) Bipolar = 0.5, natural(0.2) Unipolar = 0.6;
        // diff = -0.1.
        assert!(
            (bu - (-0.1)).abs() < TOLERANCE,
            "bu expected -0.1, got {bu}"
        );
        // UU: natural(0.5) = 0.75, natural(0.2) = 0.6; diff = 0.15.
        assert!((uu - 0.15).abs() < TOLERANCE, "uu expected 0.15, got {uu}");
    }

    #[test]
    fn bidirectional_mixed_bipolar_center_plus_unipolar_idle_centers() {
        // Bipolar primary at center (encoded 0, natural 0) + unipolar
        // secondary at idle (encoded -1, natural 0). Pre-fix returned
        // encoded 0 - (-1) = 1 (full deflection), spuriously claiming
        // input where there was none. Natural diff is 0 (centered).
        assert!(
            merge_axes(
                0.0,
                -1.0,
                MergeOp::Bidirectional,
                AxisPolarity::Bipolar,
                AxisPolarity::Unipolar
            )
            .abs()
                < TOLERANCE
        );
    }

    // -- Average: encoded math is self-correcting via the GUI's natural
    // domain remap, so polarity is ignored in the merge_axes computation.

    #[test]
    fn average_unchanged_for_unipolar_pair() {
        // Two unipolar pedals at idle (encoded -1, -1):
        // midpoint(-1, -1) = -1. The GUI re-interprets this as natural
        // 0 (empty) via into_natural_domain.
        assert!((merge_axes(-1.0, -1.0, MergeOp::Average, UU.0, UU.1) - (-1.0)).abs() < TOLERANCE);
    }

    #[test]
    fn average_unipolar_pair_at_full_press_round_trips_to_full_natural() {
        // Two unipolar pedals fully pressed (encoded 1, 1):
        // midpoint(1, 1) = 1. The GUI re-interprets this as natural 1
        // (full bar) via into_natural_domain.
        assert!((merge_axes(1.0, 1.0, MergeOp::Average, UU.0, UU.1) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn average_unchanged_for_mixed_polarity() {
        // Encoded math identical regardless of polarity hints.
        let bb = merge_axes(0.5, 0.2, MergeOp::Average, BB.0, BB.1);
        let ub = merge_axes(
            0.5,
            0.2,
            MergeOp::Average,
            AxisPolarity::Unipolar,
            AxisPolarity::Bipolar,
        );
        assert!((bb - 0.35).abs() < TOLERANCE);
        assert!((ub - 0.35).abs() < TOLERANCE);
    }
}
