//! Curve conversion edits.
use super::super::CurveType;
use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};
pub(crate) fn reconstruct_curve(curve: &ResponseCurve) -> Result<ResponseCurve, String> {
    if super::super::state::extract_anchors(curve)
        .iter()
        .any(|(x, y)| !x.is_finite() || !y.is_finite())
    {
        return Err("Curve points must be finite.".into());
    }
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric } => {
            ResponseCurve::piecewise_linear(points.clone(), *symmetric).map_err(|e| e.to_string())
        }
        ResponseCurve::CubicSpline { points, symmetric } => {
            ResponseCurve::cubic_spline(points.clone(), *symmetric).map_err(|e| e.to_string())
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => ResponseCurve::cubic_bezier(segments.clone(), *symmetric).map_err(|e| e.to_string()),
    }
}

#[must_use]
pub(crate) fn default_identity_curve(curve: &ResponseCurve) -> ResponseCurve {
    match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. } => {
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], *symmetric)
                .unwrap_or_else(|_| {
                    ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                        .expect("hardcoded identity is valid")
                })
        }
        ResponseCurve::CubicSpline { symmetric, .. } => {
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], *symmetric)
                .unwrap_or_else(|_| {
                    ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                        .expect("hardcoded identity is valid")
                })
        }
        ResponseCurve::CubicBezier { symmetric, .. } => {
            let segs = symmetric_bezier_identity(*symmetric);
            ResponseCurve::cubic_bezier(segs, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::cubic_bezier(symmetric_bezier_identity(false), false)
                    .expect("hardcoded bezier identity is valid")
            })
        }
    }
}

pub(super) fn symmetric_bezier_identity(symmetric: bool) -> Vec<BezierSegment> {
    if symmetric {
        vec![
            BezierSegment {
                start: (-1.0, -1.0),
                control1: (-2.0 / 3.0, -2.0 / 3.0),
                control2: (-1.0 / 3.0, -1.0 / 3.0),
                end: (0.0, 0.0),
            },
            BezierSegment {
                start: (0.0, 0.0),
                control1: (1.0 / 3.0, 1.0 / 3.0),
                control2: (2.0 / 3.0, 2.0 / 3.0),
                end: (1.0, 1.0),
            },
        ]
    } else {
        vec![BezierSegment {
            start: (-1.0, -1.0),
            control1: (-1.0 / 3.0, -1.0 / 3.0),
            control2: (1.0 / 3.0, 1.0 / 3.0),
            end: (1.0, 1.0),
        }]
    }
}

// ---------------------------------------------------------------------------
// Type conversion
// ---------------------------------------------------------------------------

#[must_use]
pub(crate) fn convert_curve_type(
    curve: &ResponseCurve,
    target: CurveType,
) -> Option<ResponseCurve> {
    let symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };
    match target {
        CurveType::PiecewiseLinear => {
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], symmetric)
                .ok()
        }
        CurveType::CubicSpline => {
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], symmetric).ok()
        }
        CurveType::CubicBezier => {
            ResponseCurve::cubic_bezier(symmetric_bezier_identity(symmetric), symmetric).ok()
        }
    }
}
