//! Curve point edits.
use super::bezier::update_bezier_point;
use inputforge_core::processing::curves::ResponseCurve;
// Preserve the strict interpolation gap while dragging.
const MIN_X_GAP: f64 = 0.001;
pub(crate) fn adjacent_x_bounds(curve: &ResponseCurve, index: usize) -> (f64, f64) {
    let symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => {
            let count = points.len();
            if index == 0 {
                return (points[0].0, points[0].0);
            }
            if index == count - 1 {
                return (points[count - 1].0, points[count - 1].0);
            }
            if symmetric && count % 2 == 1 && index == count / 2 {
                return (0.0, 0.0);
            }
            // Valid imported curves may have gaps smaller than the editor step.
            (
                (points[index - 1].0 + MIN_X_GAP).min(points[index].0),
                (points[index + 1].0 - MIN_X_GAP).max(points[index].0),
            )
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let seg_idx = index / 4;
            let local = index % 4;
            let last_seg = segments.len().saturating_sub(1);
            if seg_idx == 0 && local == 0 {
                return (-1.0, -1.0);
            }
            if seg_idx == last_seg && local == 3 {
                return (1.0, 1.0);
            }
            (-1.0, 1.0)
        }
    }
}

pub(crate) fn update_point_in_curve(
    curve: &mut ResponseCurve,
    index: usize,
    new_pos: (f64, f64),
    bounds: (f64, f64),
) {
    let new_x = new_pos.0.clamp(bounds.0, bounds.1);
    let new_y = new_pos.1.clamp(-1.0, 1.0);
    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            if *symmetric && points.len() % 2 == 1 && index == points.len() / 2 {
                return;
            }
            if let Some(pt) = points.get_mut(index) {
                pt.0 = new_x;
                pt.1 = new_y;
            }
            if *symmetric {
                let count = points.len();
                let mirror_idx = count - 1 - index;
                if mirror_idx != index
                    && let Some(mirror_pt) = points.get_mut(mirror_idx)
                {
                    mirror_pt.0 = -new_x;
                    mirror_pt.1 = -new_y;
                }
            }
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            update_bezier_point(segments, *symmetric, index, new_x, new_y);
        }
    }
}
