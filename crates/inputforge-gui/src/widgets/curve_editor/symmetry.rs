// Rust guideline compliant 2026-03-04

//! Symmetry enforcement for the curve editor.
//!
//! Handles enabling and disabling antisymmetric mode on response curves,
//! mirroring positive-half points to the negative side through the origin.

use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

/// Apply a symmetry change to the curve, returning a validated result.
///
/// When **enabling** symmetry, restructures the curve to be antisymmetric
/// through the origin by mirroring the positive-half points to the negative
/// side, matching `JoystickGremlin` `_enforce_symmetry()` behavior.
///
/// When **disabling** symmetry, simply clears the flag — all points (both
/// sides) are kept as-is.
///
/// Returns `None` when the post-change state fails validation.
pub(super) fn apply_symmetry(curve: &ResponseCurve, symmetric: bool) -> Option<ResponseCurve> {
    if symmetric {
        // Enabling: enforce antisymmetry.
        enforce_symmetry(curve)
    } else {
        // Disabling: mutate the flag in-place without cloning or re-validating.
        let mut result = curve.clone();
        result.set_symmetric(false);
        Some(result)
    }
}

/// Enforce antisymmetry on a curve: `f(-x) = -f(x)`.
///
/// Takes the positive-half points (x >= 0), mirrors them to create the
/// negative side, and ensures the origin is included. Matches
/// `JoystickGremlin` `_enforce_symmetry()`.
fn enforce_symmetry(curve: &ResponseCurve) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. } => {
            let pts = enforce_symmetry_points(points);
            ResponseCurve::piecewise_linear(pts, true).ok()
        }
        ResponseCurve::CubicSpline { points, .. } => {
            let pts = enforce_symmetry_points(points);
            ResponseCurve::cubic_spline(pts, true).ok()
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let segs = enforce_symmetry_bezier(segments);
            ResponseCurve::cubic_bezier(segs, true).ok()
        }
    }
}

/// Build a full antisymmetric point set from existing points.
///
/// Keeps points with x >= 0, mirrors them to the negative side, and
/// ensures the origin (0, 0) is present. If no positive-side points
/// exist, falls back to a minimal identity.
fn enforce_symmetry_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    // Collect positive-half points (x >= 0), sorted by x.
    let mut positive: Vec<(f64, f64)> = points.iter().filter(|(x, _)| *x >= 0.0).copied().collect();
    positive.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Ensure origin is present.
    if positive.is_empty() || positive[0].0 > 0.0 {
        positive.insert(0, (0.0, 0.0));
    } else {
        // Lock origin y to 0 for antisymmetry.
        positive[0].1 = 0.0;
    }

    // Ensure at least (0,0) and (1,1).
    if positive.len() < 2 {
        positive.push((1.0, 1.0));
    }

    // Mirror positive points (excluding origin) to negative side.
    let mut result: Vec<(f64, f64)> = positive
        .iter()
        .filter(|(x, _)| *x > 0.0)
        .map(|(x, y)| (-x, -y))
        .collect();
    result.reverse();
    result.extend_from_slice(&positive);
    result
}

/// Build a full antisymmetric bezier segment set from existing segments.
///
/// Keeps segments in the positive domain and mirrors them to the negative
/// side. If no positive segments exist, creates a default symmetric pair.
fn enforce_symmetry_bezier(segments: &[BezierSegment]) -> Vec<BezierSegment> {
    // Collect segments with start.x >= 0.
    let positive: Vec<_> = segments
        .iter()
        .filter(|s| s.start.0 >= 0.0)
        .cloned()
        .collect();

    let positive = if positive.is_empty() {
        // Fallback: create a default positive segment.
        vec![BezierSegment {
            start: (0.0, 0.0),
            control1: (1.0 / 3.0, 1.0 / 3.0),
            control2: (2.0 / 3.0, 2.0 / 3.0),
            end: (1.0, 1.0),
        }]
    } else {
        positive
    };

    // Mirror positive segments to create negative side.
    let mut mirrored: Vec<BezierSegment> = positive
        .iter()
        .rev()
        .map(|seg| BezierSegment {
            start: (-seg.end.0, -seg.end.1),
            control1: (-seg.control2.0, -seg.control2.1),
            control2: (-seg.control1.0, -seg.control1.1),
            end: (-seg.start.0, -seg.start.1),
        })
        .collect();
    mirrored.extend_from_slice(&positive);
    mirrored
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_symmetry_enforces_antisymmetric_points() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let result = apply_symmetry(&curve, true);
        assert!(
            result.is_some(),
            "enabling symmetry must succeed for identity"
        );
        if let Some(ResponseCurve::PiecewiseLinear { points, symmetric }) = result {
            assert!(symmetric);
            // Must have origin and mirrored points on both sides.
            assert!(points.len() >= 3);
            // Origin must be at (0, 0).
            let center = points.iter().find(|(x, _)| x.abs() < f64::EPSILON);
            assert!(center.is_some(), "origin must be present");
            assert!(
                (center.unwrap().1).abs() < f64::EPSILON,
                "origin y must be 0"
            );
        }
    }

    #[test]
    fn apply_symmetry_two_point_default_curve() {
        // The default identity curve [(-1,-1), (1,1)] must produce a valid
        // symmetric curve with origin and mirrored points on both sides.
        let curve = ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
        let result = apply_symmetry(&curve, true);
        assert!(
            result.is_some(),
            "enabling symmetry on 2-point default curve must succeed"
        );
        if let Some(ResponseCurve::PiecewiseLinear { points, symmetric }) = result {
            assert!(symmetric);
            assert!(
                points.len() >= 3,
                "symmetric curve must have at least 3 points (neg, origin, pos), got {}",
                points.len()
            );
            // First point must be negative, last must be positive.
            assert!(points[0].0 < 0.0, "first point must be negative x");
            assert!(
                points[points.len() - 1].0 > 0.0,
                "last point must be positive x"
            );
        }
    }

    #[test]
    fn apply_symmetry_disable_keeps_all_points() {
        // Symmetric curve with full-range points; disabling just clears the flag.
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (0.5, 0.2), (1.0, 1.0)],
            true,
        )
        .unwrap();
        let result = apply_symmetry(&curve, false);
        assert!(result.is_some(), "disabling symmetry must succeed");
        if let Some(ResponseCurve::PiecewiseLinear { points, symmetric }) = result {
            assert!(!symmetric);
            // All original points must be preserved.
            assert_eq!(points.len(), 4);
        }
    }

    #[test]
    fn enforce_symmetry_points_produces_antisymmetric() {
        let points = vec![(-1.0, -0.8), (0.0, 0.1), (0.5, 0.3), (1.0, 1.0)];
        let result = enforce_symmetry_points(&points);
        // Must have mirrored positive side to negative and fixed origin y to 0.
        assert!(result.len() >= 5);
        // Check antisymmetry: for each positive point, a mirrored negative must exist.
        for &(x, y) in &result {
            if x.abs() > f64::EPSILON {
                let mirror = result.iter().find(|(mx, _)| (mx + x).abs() < f64::EPSILON);
                assert!(mirror.is_some(), "mirror of ({x}, {y}) must exist");
                let (_, my) = mirror.unwrap();
                assert!(
                    (my + y).abs() < f64::EPSILON,
                    "mirror y must be -{y}, got {my}"
                );
            }
        }
    }
}
