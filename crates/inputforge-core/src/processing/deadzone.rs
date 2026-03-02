// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

use super::lerp_range;

/// Four-parameter deadzone configuration.
///
/// Defines five zones on the [-1, 1] axis:
/// - Below `low`: saturated at -1.0
/// - [`low`, `center_low`]: linearly maps to [-1.0, 0.0]
/// - [`center_low`, `center_high`]: dead center, returns 0.0
/// - [`center_high`, `high`]: linearly maps to [0.0, 1.0]
/// - Above `high`: saturated at 1.0
///
/// Invariant: `low < center_low <= center_high < high`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeadzoneConfig {
    low: f64,
    center_low: f64,
    center_high: f64,
    high: f64,
}

/// Raw deserialization target for [`DeadzoneConfig`].
#[derive(Deserialize)]
struct DeadzoneConfigRaw {
    low: f64,
    center_low: f64,
    center_high: f64,
    high: f64,
}

impl TryFrom<DeadzoneConfigRaw> for DeadzoneConfig {
    type Error = EngineError;

    fn try_from(raw: DeadzoneConfigRaw) -> Result<Self> {
        Self::new(raw.low, raw.center_low, raw.center_high, raw.high)
    }
}

impl<'de> Deserialize<'de> for DeadzoneConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = DeadzoneConfigRaw::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

impl Default for DeadzoneConfig {
    fn default() -> Self {
        Self::new(-1.0, -0.05, 0.05, 1.0).expect("default deadzone config is valid")
    }
}

impl DeadzoneConfig {
    /// Create a validated deadzone configuration.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] when the invariant
    /// `low < center_low <= center_high < high` is violated.
    pub fn new(low: f64, center_low: f64, center_high: f64, high: f64) -> Result<Self> {
        if low >= center_low {
            return Err(EngineError::InvalidConfig {
                reason: format!("low ({low}) must be less than center_low ({center_low})"),
            });
        }
        if center_low > center_high {
            return Err(EngineError::InvalidConfig {
                reason: format!("center_low ({center_low}) must be <= center_high ({center_high})"),
            });
        }
        if center_high >= high {
            return Err(EngineError::InvalidConfig {
                reason: format!("center_high ({center_high}) must be less than high ({high})"),
            });
        }

        Ok(Self {
            low,
            center_low,
            center_high,
            high,
        })
    }

    /// Return the low threshold.
    #[must_use]
    pub fn low(&self) -> f64 {
        self.low
    }

    /// Return the center-low threshold.
    #[must_use]
    pub fn center_low(&self) -> f64 {
        self.center_low
    }

    /// Return the center-high threshold.
    #[must_use]
    pub fn center_high(&self) -> f64 {
        self.center_high
    }

    /// Return the high threshold.
    #[must_use]
    pub fn high(&self) -> f64 {
        self.high
    }

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
        let mid = f64::midpoint(dz.low(), dz.center_low());
        assert!((dz.apply(mid) - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn lerp_midpoint_positive_side() {
        let dz = DeadzoneConfig::default();
        let mid = f64::midpoint(dz.center_high(), dz.high());
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

    #[test]
    fn zero_width_center_band_accepted() {
        let dz = DeadzoneConfig::new(-1.0, 0.0, 0.0, 1.0).unwrap();
        assert!((dz.apply(0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reject_low_equals_center_low() {
        let err = DeadzoneConfig::new(-1.0, -1.0, 0.0, 1.0).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_center_low_greater_than_center_high() {
        let err = DeadzoneConfig::new(-1.0, 0.1, -0.1, 1.0).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_center_high_equals_high() {
        let err = DeadzoneConfig::new(-1.0, -0.05, 1.0, 1.0).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_invalid_serde_input() {
        let json = r#"{"low":0.5,"center_low":0.5,"center_high":0.5,"high":0.5}"#;
        let result: std::result::Result<DeadzoneConfig, _> = serde_json::from_str(json);
        result.unwrap_err();
    }

    #[test]
    fn getters_return_correct_values() {
        let dz = DeadzoneConfig::new(-0.9, -0.1, 0.1, 0.9).unwrap();
        assert!((dz.low() - (-0.9)).abs() < f64::EPSILON);
        assert!((dz.center_low() - (-0.1)).abs() < f64::EPSILON);
        assert!((dz.center_high() - 0.1).abs() < f64::EPSILON);
        assert!((dz.high() - 0.9).abs() < f64::EPSILON);
    }
}
