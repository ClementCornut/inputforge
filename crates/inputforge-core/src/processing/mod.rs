// Rust guideline compliant 2026-03-02

pub mod calibration;
pub mod curves;
pub mod deadzone;
pub mod inversion;

pub use calibration::Calibration;
pub use curves::{BezierSegment, ResponseCurve, bezier_x, bezier_y};
pub use deadzone::DeadzoneConfig;
pub use inversion::{invert_axis, invert_button};

/// Linearly interpolate `value` from the range [`in_min`, `in_max`] to [`out_min`, `out_max`].
///
/// Assumes `in_min < in_max`. Does NOT clamp the result.
pub(crate) fn lerp_range(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    debug_assert!(in_min < in_max, "lerp_range requires in_min < in_max");
    let t = (value - in_min) / (in_max - in_min);
    out_min + t * (out_max - out_min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_range_min_maps_to_out_min() {
        let result = lerp_range(0.0, 0.0, 1.0, 10.0, 20.0);
        assert!((result - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lerp_range_max_maps_to_out_max() {
        let result = lerp_range(1.0, 0.0, 1.0, 10.0, 20.0);
        assert!((result - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lerp_range_midpoint() {
        let result = lerp_range(0.5, 0.0, 1.0, 10.0, 20.0);
        assert!((result - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lerp_range_negative_ranges() {
        let result = lerp_range(0.0, -1.0, 1.0, -10.0, 10.0);
        assert!((result - 0.0).abs() < f64::EPSILON);
    }
}
