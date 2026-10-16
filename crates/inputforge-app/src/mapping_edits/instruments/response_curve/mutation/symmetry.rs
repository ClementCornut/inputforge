//! Curve symmetry edits.
use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};
pub(crate) fn apply_symmetry(curve: &ResponseCurve, symmetric: bool) -> Option<ResponseCurve> {
    if symmetric {
        enforce_symmetry(curve)
    } else {
        let mut result = curve.clone();
        result.set_symmetric(false);
        Some(result)
    }
}

fn enforce_symmetry(curve: &ResponseCurve) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. } => {
            ResponseCurve::piecewise_linear(enforce_symmetry_points(points), true).ok()
        }
        ResponseCurve::CubicSpline { points, .. } => {
            ResponseCurve::cubic_spline(enforce_symmetry_points(points), true).ok()
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            ResponseCurve::cubic_bezier(enforce_symmetry_bezier(segments), true).ok()
        }
    }
}

fn enforce_symmetry_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut positive: Vec<(f64, f64)> = points.iter().filter(|(x, _)| *x >= 0.0).copied().collect();
    positive.sort_by(|a, b| a.0.total_cmp(&b.0));
    if positive.is_empty() || positive[0].0 > 0.0 {
        positive.insert(0, (0.0, 0.0));
    } else {
        positive[0].1 = 0.0;
    }
    if positive.len() < 2 {
        positive.push((1.0, 1.0));
    }
    let mut result: Vec<(f64, f64)> = positive
        .iter()
        .filter(|(x, _)| *x > 0.0)
        .map(|(x, y)| (-x, -y))
        .collect();
    result.reverse();
    result.extend_from_slice(&positive);
    result
}

fn enforce_symmetry_bezier(segments: &[BezierSegment]) -> Vec<BezierSegment> {
    let positive: Vec<_> = segments
        .iter()
        .filter(|s| s.start.0 >= 0.0)
        .cloned()
        .collect();
    let positive = if positive.is_empty() {
        vec![BezierSegment {
            start: (0.0, 0.0),
            control1: (1.0 / 3.0, 1.0 / 3.0),
            control2: (2.0 / 3.0, 2.0 / 3.0),
            end: (1.0, 1.0),
        }]
    } else {
        positive
    };
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
