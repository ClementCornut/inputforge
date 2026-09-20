//! Persisted physical axis behavior, independent of profiles and input backends.
use crate::types::AxisPolarity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxisSetting {
    pub code: u16,
    #[serde(default)]
    pub detected: Option<AxisPolarity>,
    #[serde(default)]
    pub override_polarity: Option<AxisPolarity>,
}
impl AxisSetting {
    #[must_use]
    pub fn effective(&self) -> AxisPolarity {
        self.override_polarity.or(self.detected).unwrap_or_default()
    }
    /// Classify one trustworthy normalized resting sample. Existing detections stay fixed.
    pub fn detect(&mut self, value: f64) {
        if self.detected.is_none() && value.is_finite() {
            // ponytail: resting-position heuristic; manual override handles held controls.
            self.detected = Some(if value < -0.5 {
                AxisPolarity::Unipolar
            } else {
                AxisPolarity::Bipolar
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detection_is_one_shot_and_override_survives_round_trip() {
        let mut setting = AxisSetting {
            code: 3,
            detected: None,
            override_polarity: None,
        };
        setting.detect(-1.0);
        setting.detect(0.0);
        assert_eq!(setting.effective(), AxisPolarity::Unipolar);
        setting.override_polarity = Some(AxisPolarity::Bipolar);
        let saved = toml::to_string(&setting).unwrap();
        let restored: AxisSetting = toml::from_str(&saved).unwrap();
        assert_eq!(restored.detected, Some(AxisPolarity::Unipolar));
        assert_eq!(restored.effective(), AxisPolarity::Bipolar);
    }
}
