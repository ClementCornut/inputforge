// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use super::lerp_range;

/// Five-value band calibration for mapping raw physical input to normalized [-1, 1].
///
/// The center is a band (not a point) to handle physical stick jitter around center.
/// No `Default` impl because physical min/max/center values are device-specific.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    pub physical_min: f64,
    pub physical_center_low: f64,
    pub physical_center_high: f64,
    pub physical_max: f64,
    pub enabled: bool,
}

impl Calibration {
    /// Apply calibration to a raw physical input value.
    ///
    /// Returns the value unchanged when `enabled` is false.
    #[must_use]
    pub fn apply(&self, value: f64) -> f64 {
        if !self.enabled {
            return value;
        }

        if value <= self.physical_min {
            -1.0
        } else if value <= self.physical_center_low {
            lerp_range(
                value,
                self.physical_min,
                self.physical_center_low,
                -1.0,
                0.0,
            )
        } else if value <= self.physical_center_high {
            0.0
        } else if value <= self.physical_max {
            lerp_range(
                value,
                self.physical_center_high,
                self.physical_max,
                0.0,
                1.0,
            )
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_calibration() -> Calibration {
        Calibration {
            physical_min: -32768.0,
            physical_center_low: -100.0,
            physical_center_high: 100.0,
            physical_max: 32767.0,
            enabled: true,
        }
    }

    #[test]
    fn min_maps_to_neg_one() {
        let cal = test_calibration();
        assert!((cal.apply(-32768.0) - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn max_maps_to_one() {
        let cal = test_calibration();
        assert!((cal.apply(32767.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn center_band_maps_to_zero() {
        let cal = test_calibration();
        assert!((cal.apply(0.0) - 0.0).abs() < f64::EPSILON);
        assert!((cal.apply(-50.0) - 0.0).abs() < f64::EPSILON);
        assert!((cal.apply(99.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn disabled_passes_through() {
        let mut cal = test_calibration();
        cal.enabled = false;
        assert!((cal.apply(42.0) - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn below_min_clamps() {
        let cal = test_calibration();
        assert!((cal.apply(-40000.0) - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn above_max_clamps() {
        let cal = test_calibration();
        assert!((cal.apply(40000.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn midpoint_negative_side() {
        let cal = test_calibration();
        // Midpoint of [min=-32768, center_low=-100] → maps to -0.5
        let mid = (cal.physical_min + cal.physical_center_low) / 2.0;
        assert!((cal.apply(mid) - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn midpoint_positive_side() {
        let cal = test_calibration();
        // Midpoint of [center_high=100, max=32767] → maps to 0.5
        let mid = (cal.physical_center_high + cal.physical_max) / 2.0;
        assert!((cal.apply(mid) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn serde_roundtrip() {
        let cal = test_calibration();
        let json = serde_json::to_string(&cal).unwrap();
        let back: Calibration = serde_json::from_str(&json).unwrap();
        assert_eq!(cal, back);
    }
}
