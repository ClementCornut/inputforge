// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

/// A response curve that transforms axis input to output.
///
/// Constructed via [`ResponseCurve::piecewise_linear`], [`ResponseCurve::cubic_spline`],
/// or [`ResponseCurve::cubic_bezier`], which validate invariants at construction time.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponseCurve {
    PiecewiseLinear {
        points: Vec<(f64, f64)>,
        symmetric: bool,
    },
    CubicSpline {
        points: Vec<(f64, f64)>,
        symmetric: bool,
    },
    CubicBezier {
        segments: Vec<BezierSegment>,
        symmetric: bool,
    },
}

/// Raw deserialization target for [`ResponseCurve`].
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ResponseCurveRaw {
    PiecewiseLinear {
        points: Vec<(f64, f64)>,
        symmetric: bool,
    },
    CubicSpline {
        points: Vec<(f64, f64)>,
        symmetric: bool,
    },
    CubicBezier {
        segments: Vec<BezierSegment>,
        symmetric: bool,
    },
}

impl TryFrom<ResponseCurveRaw> for ResponseCurve {
    type Error = EngineError;

    fn try_from(raw: ResponseCurveRaw) -> Result<Self> {
        match raw {
            ResponseCurveRaw::PiecewiseLinear { points, symmetric } => {
                Self::piecewise_linear(points, symmetric)
            }
            ResponseCurveRaw::CubicSpline { points, symmetric } => {
                Self::cubic_spline(points, symmetric)
            }
            ResponseCurveRaw::CubicBezier {
                segments,
                symmetric,
            } => Self::cubic_bezier(segments, symmetric),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseCurve {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = ResponseCurveRaw::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

/// A single cubic bezier segment defined by four control points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BezierSegment {
    pub start: (f64, f64),
    pub control1: (f64, f64),
    pub control2: (f64, f64),
    pub end: (f64, f64),
}

/// Validate that points satisfy the response curve invariants.
///
/// Requires >= 2 points, strictly increasing x, and x >= 0 when symmetric.
fn validate_points(points: &[(f64, f64)], symmetric: bool, kind: &str) -> Result<()> {
    if points.len() < 2 {
        return Err(EngineError::InvalidConfig {
            reason: format!("{kind} requires at least 2 points, got {}", points.len()),
        });
    }
    for window in points.windows(2) {
        if window[0].0 >= window[1].0 {
            return Err(EngineError::InvalidConfig {
                reason: format!(
                    "{kind} points must have strictly increasing x values, \
                     found x={} followed by x={}",
                    window[0].0, window[1].0
                ),
            });
        }
    }
    if symmetric {
        for &(x, _) in points {
            if x < 0.0 {
                return Err(EngineError::InvalidConfig {
                    reason: format!("{kind} with symmetric=true requires all x >= 0, found x={x}"),
                });
            }
        }
    }
    Ok(())
}

impl ResponseCurve {
    /// Create a validated piecewise linear response curve.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] when:
    /// - fewer than 2 points are provided
    /// - x values are not strictly increasing
    /// - symmetric is true but some x < 0
    pub fn piecewise_linear(points: Vec<(f64, f64)>, symmetric: bool) -> Result<Self> {
        validate_points(&points, symmetric, "PiecewiseLinear")?;
        Ok(Self::PiecewiseLinear { points, symmetric })
    }

    /// Create a validated cubic spline response curve.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] when:
    /// - fewer than 2 points are provided
    /// - x values are not strictly increasing
    /// - symmetric is true but some x < 0
    pub fn cubic_spline(points: Vec<(f64, f64)>, symmetric: bool) -> Result<Self> {
        validate_points(&points, symmetric, "CubicSpline")?;
        Ok(Self::CubicSpline { points, symmetric })
    }

    /// Create a validated cubic bezier response curve.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] when segments is empty.
    pub fn cubic_bezier(segments: Vec<BezierSegment>, symmetric: bool) -> Result<Self> {
        if segments.is_empty() {
            return Err(EngineError::InvalidConfig {
                reason: "CubicBezier requires at least 1 segment".to_owned(),
            });
        }
        Ok(Self::CubicBezier {
            segments,
            symmetric,
        })
    }

    /// Evaluate the curve at the given input value.
    #[must_use]
    pub fn evaluate(&self, input: f64) -> f64 {
        match self {
            Self::PiecewiseLinear { points, symmetric } => {
                let pts = maybe_mirror_points(points, *symmetric);
                evaluate_piecewise_linear(&pts, input)
            }
            Self::CubicSpline { points, symmetric } => {
                let pts = maybe_mirror_points(points, *symmetric);
                evaluate_cubic_spline(&pts, input)
            }
            Self::CubicBezier {
                segments,
                symmetric,
            } => {
                let segs = if *symmetric {
                    mirror_bezier_segments(segments)
                } else {
                    segments.clone()
                };
                evaluate_cubic_bezier(&segs, input)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Piecewise linear
// ---------------------------------------------------------------------------

fn evaluate_piecewise_linear(points: &[(f64, f64)], input: f64) -> f64 {
    // Invariant: validated at construction to have >= 2 points
    if input <= points[0].0 {
        return points[0].1;
    }
    if input >= points[points.len() - 1].0 {
        return points[points.len() - 1].1;
    }

    for window in points.windows(2) {
        let (x0, y0) = window[0];
        let (x1, y1) = window[1];
        if input <= x1 {
            let t = (input - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }

    points[points.len() - 1].1
}

// ---------------------------------------------------------------------------
// Cubic spline (natural, Thomas algorithm)
// ---------------------------------------------------------------------------

/// Coefficients for one segment of a natural cubic spline.
struct SplineCoeffs {
    poly_a: f64,
    poly_b: f64,
    poly_c: f64,
    poly_d: f64,
    x_start: f64,
}

/// Compute natural cubic spline coefficients using the Thomas algorithm.
fn compute_spline_coefficients(points: &[(f64, f64)]) -> Vec<SplineCoeffs> {
    let seg_count = points.len() - 1;

    // Segment widths and slope differences
    let widths: Vec<f64> = (0..seg_count)
        .map(|i| points[i + 1].0 - points[i].0)
        .collect();
    let alpha: Vec<f64> = (1..seg_count)
        .map(|i| {
            3.0 * ((points[i + 1].1 - points[i].1) / widths[i]
                - (points[i].1 - points[i - 1].1) / widths[i - 1])
        })
        .collect();

    // Forward elimination
    let mut lower_diag = vec![1.0; seg_count + 1];
    let mut mu = vec![0.0; seg_count + 1];
    let mut rhs = vec![0.0; seg_count + 1];

    for i in 1..seg_count {
        lower_diag[i] = 2.0 * (points[i + 1].0 - points[i - 1].0) - widths[i - 1] * mu[i - 1];
        mu[i] = widths[i] / lower_diag[i];
        rhs[i] = (alpha[i - 1] - widths[i - 1] * rhs[i - 1]) / lower_diag[i];
    }

    // Back substitution for second derivatives
    let mut second_deriv = vec![0.0; seg_count + 1];
    for j in (0..seg_count).rev() {
        second_deriv[j] = rhs[j] - mu[j] * second_deriv[j + 1];
    }

    // Convert to polynomial coefficients
    (0..seg_count)
        .map(|i| {
            let poly_a = points[i].1;
            let poly_b = (points[i + 1].1 - points[i].1) / widths[i]
                - widths[i] * (2.0 * second_deriv[i] + second_deriv[i + 1]) / 3.0;
            let poly_d = (second_deriv[i + 1] - second_deriv[i]) / (3.0 * widths[i]);
            SplineCoeffs {
                poly_a,
                poly_b,
                poly_c: second_deriv[i],
                poly_d,
                x_start: points[i].0,
            }
        })
        .collect()
}

fn evaluate_cubic_spline(points: &[(f64, f64)], input: f64) -> f64 {
    // Invariant: validated at construction to have >= 2 points
    if input <= points[0].0 {
        return points[0].1;
    }
    if input >= points[points.len() - 1].0 {
        return points[points.len() - 1].1;
    }

    let coeffs = compute_spline_coefficients(points);

    for (i, coeff) in coeffs.iter().enumerate() {
        if input <= points[i + 1].0 {
            let dx = input - coeff.x_start;
            return coeff.poly_a
                + coeff.poly_b * dx
                + coeff.poly_c * dx * dx
                + coeff.poly_d * dx * dx * dx;
        }
    }

    points[points.len() - 1].1
}

// ---------------------------------------------------------------------------
// Cubic bezier (Newton + bisection)
// ---------------------------------------------------------------------------

fn bezier_x(seg: &BezierSegment, t: f64) -> f64 {
    let u = 1.0 - t;
    u * u * u * seg.start.0
        + 3.0 * u * u * t * seg.control1.0
        + 3.0 * u * t * t * seg.control2.0
        + t * t * t * seg.end.0
}

fn bezier_y(seg: &BezierSegment, t: f64) -> f64 {
    let u = 1.0 - t;
    u * u * u * seg.start.1
        + 3.0 * u * u * t * seg.control1.1
        + 3.0 * u * t * t * seg.control2.1
        + t * t * t * seg.end.1
}

fn bezier_dx(seg: &BezierSegment, t: f64) -> f64 {
    let u = 1.0 - t;
    3.0 * u * u * (seg.control1.0 - seg.start.0)
        + 6.0 * u * t * (seg.control2.0 - seg.control1.0)
        + 3.0 * t * t * (seg.end.0 - seg.control2.0)
}

/// Find parameter t such that `bezier_x(seg, t) ≈ x`.
///
/// Uses Newton's method (8 iterations) with bisection fallback (50 iterations).
fn find_t_for_x(seg: &BezierSegment, x: f64) -> f64 {
    // Newton's method
    let mut t = 0.5;
    // 8 iterations of Newton's method
    for _ in 0..8 {
        let dx = bezier_dx(seg, t);
        if dx.abs() < 1e-12 {
            break;
        }
        t -= (bezier_x(seg, t) - x) / dx;
        t = t.clamp(0.0, 1.0);
    }

    // Bisection fallback if Newton didn't converge
    if (bezier_x(seg, t) - x).abs() > 1e-6 {
        let mut lo = 0.0_f64;
        let mut hi = 1.0_f64;
        // 50 iterations of bisection
        for _ in 0..50 {
            t = f64::midpoint(lo, hi);
            if bezier_x(seg, t) < x {
                lo = t;
            } else {
                hi = t;
            }
        }
    }

    t
}

fn evaluate_cubic_bezier(segments: &[BezierSegment], input: f64) -> f64 {
    // Invariant: validated at construction to have >= 1 segment
    if input <= segments[0].start.0 {
        return segments[0].start.1;
    }
    let last = &segments[segments.len() - 1];
    if input >= last.end.0 {
        return last.end.1;
    }

    for seg in segments {
        if input <= seg.end.0 {
            let t = find_t_for_x(seg, input);
            return bezier_y(seg, t);
        }
    }

    last.end.1
}

// ---------------------------------------------------------------------------
// Symmetry support
// ---------------------------------------------------------------------------

/// If `symmetric` is true, mirror positive-side points to create the negative side.
///
/// Produces antisymmetric behavior: f(-x) = -f(x).
/// Assumes points are defined for x >= 0 (including the origin).
fn maybe_mirror_points(points: &[(f64, f64)], symmetric: bool) -> Vec<(f64, f64)> {
    if !symmetric {
        return points.to_vec();
    }

    let mut result: Vec<(f64, f64)> = points
        .iter()
        .filter(|(x, _)| *x > 0.0)
        .map(|(x, y)| (-x, -y))
        .collect();
    result.reverse();

    result.extend_from_slice(points);
    result
}

/// Mirror bezier segments for the negative side of a symmetric curve.
///
/// Produces antisymmetric behavior: reverses and negates each segment.
fn mirror_bezier_segments(segments: &[BezierSegment]) -> Vec<BezierSegment> {
    let mut mirrored: Vec<BezierSegment> = segments
        .iter()
        .rev()
        .map(|seg| BezierSegment {
            start: (-seg.end.0, -seg.end.1),
            control1: (-seg.control2.0, -seg.control2.1),
            control2: (-seg.control1.0, -seg.control1.1),
            end: (-seg.start.0, -seg.start.1),
        })
        .collect();

    mirrored.extend_from_slice(segments);
    mirrored
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    // -- Piecewise linear ---------------------------------------------------

    #[test]
    fn piecewise_identity() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        assert!((curve.evaluate(0.5) - 0.5).abs() < TOLERANCE);
        assert!((curve.evaluate(-0.5) - (-0.5)).abs() < TOLERANCE);
    }

    #[test]
    fn piecewise_s_curve_midpoint() {
        let curve = ResponseCurve::piecewise_linear(
            vec![
                (-1.0, -1.0),
                (-0.5, -0.2),
                (0.0, 0.0),
                (0.5, 0.2),
                (1.0, 1.0),
            ],
            false,
        )
        .unwrap();
        assert!((curve.evaluate(0.5) - 0.2).abs() < TOLERANCE);
        assert!((curve.evaluate(0.75) - 0.6).abs() < TOLERANCE);
    }

    #[test]
    fn piecewise_clamp_below() {
        let curve = ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
        assert!((curve.evaluate(-2.0) - (-1.0)).abs() < TOLERANCE);
    }

    #[test]
    fn piecewise_clamp_above() {
        let curve = ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
        assert!((curve.evaluate(2.0) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn piecewise_single_point_rejected() {
        let err = ResponseCurve::piecewise_linear(vec![(0.0, 0.0)], false).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    // -- Cubic spline -------------------------------------------------------

    #[test]
    fn spline_passes_through_points() {
        let points = vec![
            (-1.0, -1.0),
            (-0.5, -0.2),
            (0.0, 0.0),
            (0.5, 0.2),
            (1.0, 1.0),
        ];
        let curve = ResponseCurve::cubic_spline(points.clone(), false).unwrap();
        for &(x, y) in &points {
            assert!(
                (curve.evaluate(x) - y).abs() < TOLERANCE,
                "spline at x={x} expected {y}, got {}",
                curve.evaluate(x)
            );
        }
    }

    #[test]
    fn spline_endpoints() {
        let curve =
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
        assert!((curve.evaluate(-1.0) - (-1.0)).abs() < TOLERANCE);
        assert!((curve.evaluate(1.0) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn spline_identity_points() {
        let curve =
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
        // With 3 collinear points, natural spline should produce near-identity
        assert!((curve.evaluate(0.5) - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn spline_clamp_outside() {
        let curve =
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
        assert!((curve.evaluate(-2.0) - (-1.0)).abs() < TOLERANCE);
        assert!((curve.evaluate(2.0) - 1.0).abs() < TOLERANCE);
    }

    // -- Cubic bezier -------------------------------------------------------

    #[test]
    fn bezier_endpoints() {
        let seg = BezierSegment {
            start: (-1.0, -1.0),
            control1: (-0.5, -0.5),
            control2: (0.5, 0.5),
            end: (1.0, 1.0),
        };
        let curve = ResponseCurve::cubic_bezier(vec![seg], false).unwrap();
        assert!((curve.evaluate(-1.0) - (-1.0)).abs() < TOLERANCE);
        assert!((curve.evaluate(1.0) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn bezier_linear_control_points() {
        // Control points on a straight line -> linear output
        let seg = BezierSegment {
            start: (0.0, 0.0),
            control1: (1.0 / 3.0, 1.0 / 3.0),
            control2: (2.0 / 3.0, 2.0 / 3.0),
            end: (1.0, 1.0),
        };
        let curve = ResponseCurve::cubic_bezier(vec![seg], false).unwrap();
        assert!((curve.evaluate(0.5) - 0.5).abs() < TOLERANCE);
        assert!((curve.evaluate(0.25) - 0.25).abs() < TOLERANCE);
    }

    #[test]
    fn bezier_empty_segments_rejected() {
        let err = ResponseCurve::cubic_bezier(vec![], false).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    // -- Symmetry -----------------------------------------------------------

    #[test]
    fn symmetric_piecewise_antisymmetric() {
        let curve = ResponseCurve::piecewise_linear(vec![(0.0, 0.0), (0.5, 0.2), (1.0, 1.0)], true)
            .unwrap();
        for &x in &[0.25, 0.5, 0.75, 1.0] {
            let pos = curve.evaluate(x);
            let neg = curve.evaluate(-x);
            assert!(
                (pos + neg).abs() < TOLERANCE,
                "antisymmetry failed at x={x}: f(x)={pos}, f(-x)={neg}"
            );
        }
    }

    #[test]
    fn symmetric_spline_antisymmetric() {
        let curve =
            ResponseCurve::cubic_spline(vec![(0.0, 0.0), (0.5, 0.3), (1.0, 1.0)], true).unwrap();
        for &x in &[0.25, 0.5, 0.75] {
            let pos = curve.evaluate(x);
            let neg = curve.evaluate(-x);
            assert!(
                (pos + neg).abs() < TOLERANCE,
                "antisymmetry failed at x={x}: f(x)={pos}, f(-x)={neg}"
            );
        }
    }

    #[test]
    fn symmetric_bezier_antisymmetric() {
        let seg = BezierSegment {
            start: (0.0, 0.0),
            control1: (0.3, 0.1),
            control2: (0.7, 0.9),
            end: (1.0, 1.0),
        };
        let curve = ResponseCurve::cubic_bezier(vec![seg], true).unwrap();
        for &x in &[0.25, 0.5, 0.75] {
            let pos = curve.evaluate(x);
            let neg = curve.evaluate(-x);
            assert!(
                (pos + neg).abs() < TOLERANCE,
                "antisymmetry failed at x={x}: f(x)={pos}, f(-x)={neg}"
            );
        }
    }

    // -- Serde --------------------------------------------------------------

    #[test]
    fn piecewise_serde_roundtrip() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let json = serde_json::to_string(&curve).unwrap();
        assert!(json.contains("\"kind\":\"piecewise_linear\""));
        let back: ResponseCurve = serde_json::from_str(&json).unwrap();
        assert_eq!(curve, back);
    }

    #[test]
    fn spline_serde_roundtrip() {
        let curve =
            ResponseCurve::cubic_spline(vec![(0.0, 0.0), (0.5, 0.3), (1.0, 1.0)], true).unwrap();
        let json = serde_json::to_string(&curve).unwrap();
        assert!(json.contains("\"kind\":\"cubic_spline\""));
        let back: ResponseCurve = serde_json::from_str(&json).unwrap();
        assert_eq!(curve, back);
    }

    #[test]
    fn bezier_serde_roundtrip() {
        let seg = BezierSegment {
            start: (0.0, 0.0),
            control1: (0.3, 0.1),
            control2: (0.7, 0.9),
            end: (1.0, 1.0),
        };
        let curve = ResponseCurve::cubic_bezier(vec![seg], false).unwrap();
        let json = serde_json::to_string(&curve).unwrap();
        assert!(json.contains("\"kind\":\"cubic_bezier\""));
        let back: ResponseCurve = serde_json::from_str(&json).unwrap();
        assert_eq!(curve, back);
    }

    // -- Cubic spline edge cases --------------------------------------------

    #[test]
    fn spline_single_point_rejected() {
        let err = ResponseCurve::cubic_spline(vec![(0.0, 0.0)], false).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    // -- NaN input reaches fallback paths ------------------------------------

    #[test]
    fn piecewise_nan_input_returns_last_point() {
        let curve = ResponseCurve::piecewise_linear(vec![(0.0, 0.0), (1.0, 1.0)], false).unwrap();
        // NaN bypasses all comparisons, reaching the fallback return
        let result = curve.evaluate(f64::NAN);
        assert!((result - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn spline_nan_input_returns_last_point() {
        let curve =
            ResponseCurve::cubic_spline(vec![(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)], false).unwrap();
        let result = curve.evaluate(f64::NAN);
        assert!((result - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn bezier_nan_input_returns_last_endpoint() {
        let seg = BezierSegment {
            start: (0.0, 0.0),
            control1: (0.3, 0.3),
            control2: (0.7, 0.7),
            end: (1.0, 1.0),
        };
        let curve = ResponseCurve::cubic_bezier(vec![seg], false).unwrap();
        let result = curve.evaluate(f64::NAN);
        assert!((result - 1.0).abs() < TOLERANCE);
    }

    // -- Bezier Newton break + bisection fallback ----------------------------

    #[test]
    fn bezier_bisection_fallback() {
        // Control points create an S-shaped x: control1.x=1.0, control2.x=0.0
        // At t=0.5, dx/dt=0 causing Newton to break, then bisection takes over
        let seg = BezierSegment {
            start: (0.0, 0.0),
            control1: (1.0, 0.3),
            control2: (0.0, 0.7),
            end: (1.0, 1.0),
        };
        let curve = ResponseCurve::cubic_bezier(vec![seg], false).unwrap();
        // Query x=0.3 forces Newton break at dx=0 then bisection
        let result = curve.evaluate(0.3);
        // Result should be between 0 and 1 (valid y value)
        assert!(
            (0.0..=1.0).contains(&result),
            "expected y in [0,1], got {result}"
        );
    }

    // -- Validation rejection tests -----------------------------------------

    #[test]
    fn reject_non_increasing_x() {
        let err = ResponseCurve::piecewise_linear(vec![(0.0, 0.0), (0.0, 1.0)], false).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_decreasing_x() {
        let err = ResponseCurve::cubic_spline(vec![(1.0, 1.0), (0.0, 0.0)], false).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_symmetric_with_negative_x() {
        let err =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0)], true).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_empty_points_piecewise() {
        let err = ResponseCurve::piecewise_linear(vec![], false).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_empty_points_spline() {
        let err = ResponseCurve::cubic_spline(vec![], false).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn reject_invalid_serde_input() {
        let json = r#"{"kind":"piecewise_linear","points":[[0.0,0.0]],"symmetric":false}"#;
        let result: std::result::Result<ResponseCurve, _> = serde_json::from_str(json);
        result.unwrap_err();
    }
}
