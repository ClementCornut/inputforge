// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use super::lerp_range;

/// Four-parameter deadzone configuration.
///
/// Defines five zones on the [-1, 1] axis:
/// - Below `low`: saturated at -1.0
/// - [`low`, `center_low`]: linearly maps to [-1.0, 0.0]
/// - [`center_low`, `center_high`]: dead center, returns 0.0
/// - [`center_high`, `high`]: linearly maps to [0.0, 1.0]
/// - Above `high`: saturated at 1.0
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeadzoneConfig {
    pub low: f64,
    pub center_low: f64,
    pub center_high: f64,
    pub high: f64,
}

impl Default for DeadzoneConfig {
    fn default() -> Self {
        Self {
            low: -1.0,
            // 5% center deadzone band
            center_low: -0.05,
            center_high: 0.05,
            high: 1.0,
        }
    }
}

impl DeadzoneConfig {
    /// Apply the deadzone to a normalized input value.
    #[must_use]
    pub fn apply(&self, value: f64) -> f64 {
        if value < self.low {
            -1.0
        } else if value <= self.center_low {
            lerp_range(value, self.low, self.center_low, -1.0, 0.0)
        } else if value <= self.center_high {
            0.0
        } else if value <= self.high {
            lerp_range(value, self.center_high, self.high, 0.0, 1.0)
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_returns_zero() {
        let dz = DeadzoneConfig::default();
        assert!((dz.apply(0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn below_low_returns_neg_one() {
        let dz = DeadzoneConfig::default();
        assert!((dz.apply(-1.5) - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn above_high_returns_one() {
        let dz = DeadzoneConfig::default();
        assert!((dz.apply(1.5) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn at_low_returns_neg_one() {
        let dz = DeadzoneConfig::default();
        assert!((dz.apply(-1.0) - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn at_high_returns_one() {
        let dz = DeadzoneConfig::default();
        assert!((dz.apply(1.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn center_band_all_zero() {
        let dz = DeadzoneConfig::default();
        assert!((dz.apply(-0.03) - 0.0).abs() < f64::EPSILON);
        assert!((dz.apply(0.03) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lerp_midpoint_negative_side() {
        let dz = DeadzoneConfig::default();
        // Midpoint of [low=-1.0, center_low=-0.05] is -0.525
        let mid = (dz.low + dz.center_low) / 2.0;
        assert!((dz.apply(mid) - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn lerp_midpoint_positive_side() {
        let dz = DeadzoneConfig::default();
        // Midpoint of [center_high=0.05, high=1.0] is 0.525
        let mid = (dz.center_high + dz.high) / 2.0;
        assert!((dz.apply(mid) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn default_config_full_range() {
        let dz = DeadzoneConfig::default();
        assert!((dz.apply(-1.0) - (-1.0)).abs() < f64::EPSILON);
        assert!((dz.apply(1.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn serde_roundtrip() {
        let dz = DeadzoneConfig::default();
        let json = serde_json::to_string(&dz).unwrap();
        let back: DeadzoneConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(dz, back);
    }
}
