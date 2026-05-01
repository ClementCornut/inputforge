// Rust guideline compliant 2026-05-01

//! Polarity-aware axis remapping.
//!
//! The pipeline operates entirely in the bipolar-encoded `[-1, 1]` domain
//! regardless of an axis's natural polarity. Bipolar axes rest at `0` so
//! the encoding is the natural domain. Unipolar axes (pedals, triggers)
//! rest at `-1`, full at `+1`, so the encoding does not match the
//! "0% pressed .. 100% pressed" natural domain a UI wants to display.
//!
//! [`into_natural_domain`] resolves this: it remaps a unipolar
//! encoded value to `[0, 1]` and passes a bipolar value through.
//! Both arms clamp to defend against calibration drift outputting
//! values just outside the canonical range.

use crate::types::AxisPolarity;

/// Remap a bipolar-encoded raw axis value into the polarity's natural
/// display domain.
///
/// - Bipolar: passthrough, clamped to `[-1, 1]`. The natural domain
///   already matches the encoding.
/// - Unipolar: remaps `[-1, 1]` to `[0, 1]` via `(raw + 1) / 2`,
///   clamped to `[0, 1]`. An idle unipolar pedal (encoded `-1`)
///   becomes natural `0` and a fully-pressed pedal (encoded `+1`)
///   becomes natural `1`.
#[must_use]
pub fn into_natural_domain(raw: f64, polarity: AxisPolarity) -> f64 {
    match polarity {
        AxisPolarity::Bipolar => raw.clamp(-1.0, 1.0),
        AxisPolarity::Unipolar => f64::midpoint(raw, 1.0).clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-12;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < TOLERANCE,
            "expected {expected}, got {actual}"
        );
    }

    // -- Bipolar passthrough --------------------------------------------------

    #[test]
    fn bipolar_zero_passes_through() {
        assert_close(into_natural_domain(0.0, AxisPolarity::Bipolar), 0.0);
    }

    #[test]
    fn bipolar_positive_passes_through() {
        assert_close(into_natural_domain(0.5, AxisPolarity::Bipolar), 0.5);
    }

    #[test]
    fn bipolar_negative_passes_through() {
        assert_close(into_natural_domain(-0.5, AxisPolarity::Bipolar), -0.5);
    }

    #[test]
    fn bipolar_extremes_pass_through() {
        assert_close(into_natural_domain(1.0, AxisPolarity::Bipolar), 1.0);
        assert_close(into_natural_domain(-1.0, AxisPolarity::Bipolar), -1.0);
    }

    // -- Unipolar remap -------------------------------------------------------

    #[test]
    fn unipolar_idle_remaps_to_zero() {
        // Encoded -1 (pedal at rest) -> natural 0 (0% pressed).
        assert_close(into_natural_domain(-1.0, AxisPolarity::Unipolar), 0.0);
    }

    #[test]
    fn unipolar_center_remaps_to_half() {
        // Encoded 0 (pedal half-pressed) -> natural 0.5 (50% pressed).
        assert_close(into_natural_domain(0.0, AxisPolarity::Unipolar), 0.5);
    }

    #[test]
    fn unipolar_full_press_remaps_to_one() {
        // Encoded +1 (pedal fully pressed) -> natural 1 (100% pressed).
        assert_close(into_natural_domain(1.0, AxisPolarity::Unipolar), 1.0);
    }

    // -- Out-of-range clamping ------------------------------------------------

    #[test]
    fn bipolar_above_one_clamps() {
        assert_close(into_natural_domain(1.5, AxisPolarity::Bipolar), 1.0);
    }

    #[test]
    fn bipolar_below_neg_one_clamps() {
        assert_close(into_natural_domain(-1.5, AxisPolarity::Bipolar), -1.0);
    }

    #[test]
    fn unipolar_above_one_clamps_to_one() {
        // Encoded 1.5 would naturally remap to 1.25; clamped to 1.0.
        assert_close(into_natural_domain(1.5, AxisPolarity::Unipolar), 1.0);
    }

    #[test]
    fn unipolar_below_neg_one_clamps_to_zero() {
        // Encoded -1.5 would naturally remap to -0.25; clamped to 0.0.
        assert_close(into_natural_domain(-1.5, AxisPolarity::Unipolar), 0.0);
    }

    // -- IEEE-754 quirks ------------------------------------------------------

    #[test]
    fn bipolar_negative_zero_passes_through() {
        // -0.0 == 0.0 numerically; sign passthrough is acceptable, the
        // format layer guards display.
        let result = into_natural_domain(-0.0, AxisPolarity::Bipolar);
        assert!(result.abs() < TOLERANCE, "expected ~0, got {result}");
    }
}
