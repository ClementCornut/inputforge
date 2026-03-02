// Rust guideline compliant 2026-03-02

/// Invert an axis value by negating it.
#[must_use]
pub fn invert_axis(value: f64) -> f64 {
    -value
}

/// Invert a button state by toggling it.
#[must_use]
pub fn invert_button(pressed: bool) -> bool {
    !pressed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_positive_negated() {
        assert!((invert_axis(0.5) - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn axis_negative_negated() {
        assert!((invert_axis(-0.75) - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn axis_zero_stays_zero() {
        assert!((invert_axis(0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn axis_one_to_neg_one() {
        assert!((invert_axis(1.0) - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn button_pressed_to_released() {
        assert!(!invert_button(true));
    }

    #[test]
    fn button_released_to_pressed() {
        assert!(invert_button(false));
    }
}
