// Rust guideline compliant 2026-03-03

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::address::InputAddress;

/// Normalized axis value in the range [-1.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisValue(f64);

impl AxisValue {
    /// Create a new `AxisValue`, clamping to [-1.0, 1.0].
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self(value.clamp(-1.0, 1.0))
    }

    /// Create a raw `AxisValue` without clamping (for calibration input).
    #[must_use]
    pub(crate) fn raw(value: f64) -> Self {
        Self(value)
    }

    /// Get the inner value.
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }

    /// Return a clamped copy of this value.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self::new(self.0)
    }
}

/// Hat switch direction (8-way + center).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HatDirection {
    Center,
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

/// A value read from an input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputValue {
    Axis { value: AxisValue },
    Button { pressed: bool },
    Hat { direction: HatDirection },
}

/// An event produced by a physical input device.
#[derive(Debug, Clone)]
pub struct InputEvent {
    pub source: InputAddress,
    pub value: InputValue,
    pub timestamp: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_value_new_clamps_above_one() {
        let v = AxisValue::new(1.5);
        assert!((v.value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn axis_value_new_clamps_below_neg_one() {
        let v = AxisValue::new(-2.0);
        assert!((v.value() - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn axis_value_new_preserves_in_range() {
        let v = AxisValue::new(0.5);
        assert!((v.value() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn axis_value_raw_does_not_clamp() {
        let v = AxisValue::raw(5.0);
        assert!((v.value() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn axis_value_clamped_returns_clamped_copy() {
        let v = AxisValue::raw(2.0);
        let c = v.clamped();
        assert!((c.value() - 1.0).abs() < f64::EPSILON);
        assert!((v.value() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn hat_direction_all_variants_exist() {
        let dirs = [
            HatDirection::Center,
            HatDirection::N,
            HatDirection::NE,
            HatDirection::E,
            HatDirection::SE,
            HatDirection::S,
            HatDirection::SW,
            HatDirection::W,
            HatDirection::NW,
        ];
        assert_eq!(dirs.len(), 9);
    }

    #[test]
    fn input_value_axis_serde_roundtrip() {
        let val = InputValue::Axis {
            value: AxisValue::new(0.75),
        };
        let json = serde_json::to_string(&val).unwrap();
        let back: InputValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn input_value_button_serde_roundtrip() {
        let val = InputValue::Button { pressed: true };
        let json = serde_json::to_string(&val).unwrap();
        let back: InputValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, back);
    }
}
