// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

use super::lerp_range;

/// Five-value band calibration for mapping raw physical input to normalized [-1, 1].
///
/// The center is a band (not a point) to handle physical stick jitter around center.
/// No `Default` impl because physical min/max/center values are device-specific.
///
/// Invariant: `physical_min < physical_center_low <= physical_center_high < physical_max`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Calibration {
    physical_min: f64,
    physical_center_low: f64,
    physical_center_high: f64,
    physical_max: f64,
    enabled: bool,
}

/// Raw deserialization target for [`Calibration`].
#[derive(Deserialize)]
struct CalibrationRaw {
    physical_min: f64,
    physical_center_low: f64,
    physical_center_high: f64,
    physical_max: f64,
    enabled: bool,
}

impl TryFrom<CalibrationRaw> for Calibration {
    type Error = EngineError;

    fn try_from(raw: CalibrationRaw) -> Result<Self> {
        Self::new(
            raw.physical_min,
            raw.physical_center_low,
            raw.physical_center_high,
            raw.physical_max,
            raw.enabled,
        )
    }
}

impl<'de> Deserialize<'de> for Calibration {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = CalibrationRaw::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

impl Calibration {
    /// Create a validated calibration configuration.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] when the invariant
    /// `physical_min < physical_center_low <= physical_center_high < physical_max`
    /// is violated.
    pub fn new(
        physical_min: f64,
        physical_center_low: f64,
        physical_center_high: f64,
        physical_max: f64,
        enabled: bool,
    ) -> Result<Self> {
        if physical_min >= physical_center_low {
            return Err(EngineError::InvalidConfig {
                reason: format!(
                    "physical_min ({physical_min}) must be less than physical_center_low ({physical_center_low})"
                ),
            });
        }
        if physical_center_low > physical_center_high {
            return Err(EngineError::InvalidConfig {
                reason: format!(
                    "physical_center_low ({physical_center_low}) must be <= physical_center_high ({physical_center_high})"
                ),
            });
        }
        if physical_center_high >= physical_max {
            return Err(EngineError::InvalidConfig {
                reason: format!(
                    "physical_center_high ({physical_center_high}) must be less than physical_max ({physical_max})"
                ),
            });
        }

        Ok(Self {
            physical_min,
            physical_center_low,
            physical_center_high,
            physical_max,
            enabled,
        })
    }

    /// Return the physical minimum.
    #[must_use]
    pub fn physical_min(&self) -> f64 {
        self.physical_min
    }

    /// Return the physical center-low threshold.
    #[must_use]
    pub fn physical_center_low(&self) -> f64 {
        self.physical_center_low
    }

    /// Return the physical center-high threshold.
    #[must_use]
    pub fn physical_center_high(&self) -> f64 {
        self.physical_center_high
    }

    /// Return the physical maximum.
    #[must_use]
    pub fn physical_max(&self) -> f64 {
        self.physical_max
    }

    /// Return whether calibration is enabled.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

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
        Calibration::new(-32768.0, -100.0, 100.0, 32767.0, true).unwrap()
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
        let cal = Calibration::new(-32768.0, -100.0, 100.0, 32767.0, false).unwrap();
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
        let mid = (cal.physical_min() + cal.physical_center_low()) / 2.0;
        assert!((cal.apply(mid) - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn midpoint_positive_side() {
        let cal = test_calibration();
        let mid = (cal.physical_center_high() + cal.physical_max()) / 2.0;
        assert!((cal.apply(mid) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn serde_roundtrip() {
        let cal = test_calibration();
        let json = serde_json::to_string(&cal).unwrap();
        let back: Calibration = serde_json::from_str(&json).unwrap();
        assert_eq!(cal, back);
    }

    #[test]
    fn zero_width_center_band_accepted() {
        let cal = Calibration::new(-100.0, 0.0, 0.0, 100.0, true).unwrap();
        assert!((cal.apply(0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reject_min_equals_center_low() {
        let err = Calibration::new(-100.0, -100.0, 100.0, 200.0, true).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_center_low_greater_than_center_high() {
        let err = Calibration::new(-100.0, 50.0, -50.0, 100.0, true).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_center_high_equals_max() {
        let err = Calibration::new(-100.0, -50.0, 100.0, 100.0, true).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_invalid_serde_input() {
        let json = r#"{"physical_min":0.0,"physical_center_low":0.0,"physical_center_high":0.0,"physical_max":0.0,"enabled":true}"#;
        let result: std::result::Result<Calibration, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn getters_return_correct_values() {
        let cal = Calibration::new(-500.0, -10.0, 10.0, 500.0, true).unwrap();
        assert!((cal.physical_min() - (-500.0)).abs() < f64::EPSILON);
        assert!((cal.physical_center_low() - (-10.0)).abs() < f64::EPSILON);
        assert!((cal.physical_center_high() - 10.0).abs() < f64::EPSILON);
        assert!((cal.physical_max() - 500.0).abs() < f64::EPSILON);
        assert!(cal.enabled());
    }
}
