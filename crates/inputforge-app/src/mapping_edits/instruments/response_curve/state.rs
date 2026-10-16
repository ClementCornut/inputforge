//! Response Curve state behavior.
use inputforge_core::processing::curves::ResponseCurve;

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BodyState {
    pub dragging: Option<DragInProgress>,
    pub hovered_point: Option<usize>,
    pub focused_point: Option<usize>,
    pub pre_drag_curve: Option<ResponseCurve>,
    pub cached_anchors: Vec<(f64, f64)>,
}

#[must_use]
pub(crate) fn clamp_focus(state: BodyState, new_anchor_count: usize) -> BodyState {
    let mut s = state;
    s.focused_point = match s.focused_point {
        Some(_) if new_anchor_count == 0 => None,
        Some(i) => Some(i.min(new_anchor_count - 1)),
        None => None,
    };
    s
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DragInProgress {
    pub point_index: usize,
    pub bounds: (f64, f64),
}

#[must_use]
pub(crate) fn extract_anchors(curve: &ResponseCurve) -> Vec<(f64, f64)> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => points.clone(),
        ResponseCurve::CubicBezier { segments, .. } => {
            let mut pts = Vec::with_capacity(segments.len() * 4);
            for seg in segments {
                pts.push(seg.start);
                pts.push(seg.control1);
                pts.push(seg.control2);
                pts.push(seg.end);
            }
            pts
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[expect(
        redundant_imports,
        reason = "BezierSegment is needed; ResponseCurve re-enters via glob but \
                  the explicit import keeps the test self-documenting per plan spec"
    )]
    use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

    #[test]
    fn body_state_default_is_idle() {
        let s = BodyState::default();
        assert!(s.dragging.is_none());
        assert!(s.hovered_point.is_none());
        assert!(s.focused_point.is_none());
        assert!(s.pre_drag_curve.is_none());
    }

    #[test]
    fn extract_anchors_piecewise_yields_engine_native_tuples() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let anchors = extract_anchors(&curve);
        assert_eq!(anchors.len(), 3);
        // Engine-native: tuple is (input, output), NOT (output, input).
        assert!((anchors[0].0 - (-1.0)).abs() < f64::EPSILON);
        assert!((anchors[0].1 - (-1.0)).abs() < f64::EPSILON);
        assert!((anchors[2].0 - 1.0).abs() < f64::EPSILON);
        assert!((anchors[2].1 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_anchors_bezier_interleaves_four_per_segment() {
        let curve = ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-1.0 / 3.0, -1.0 / 3.0),
                control2: (1.0 / 3.0, 1.0 / 3.0),
                end: (1.0, 1.0),
            }],
            false,
        )
        .unwrap();
        let anchors = extract_anchors(&curve);
        assert_eq!(anchors.len(), 4);
        assert!((anchors[1].0 - (-1.0 / 3.0)).abs() < 1e-9);
        assert!((anchors[2].0 - (1.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn clamp_focus_clamps_down() {
        let s = BodyState {
            focused_point: Some(4),
            ..BodyState::default()
        };
        let next = clamp_focus(s, 3);
        assert_eq!(next.focused_point, Some(2));
        assert!(next.pre_drag_curve.is_none());
    }

    #[test]
    fn clamp_focus_clears_when_empty() {
        let s = BodyState {
            focused_point: Some(0),
            ..BodyState::default()
        };
        let next = clamp_focus(s, 0);
        assert_eq!(next.focused_point, None);
    }

    #[test]
    fn clamp_focus_noop_in_range() {
        let s = BodyState {
            focused_point: Some(1),
            ..BodyState::default()
        };
        let next = clamp_focus(s, 5);
        assert_eq!(next.focused_point, Some(1));
    }
    #[test]
    fn clamping_focus_preserves_the_active_drag_baseline() {
        let baseline =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
        let state = BodyState {
            focused_point: Some(4),
            pre_drag_curve: Some(baseline.clone()),
            ..BodyState::default()
        };
        let next = clamp_focus(state, 3);
        assert_eq!(next.focused_point, Some(2));
        assert_eq!(next.pre_drag_curve, Some(baseline));
    }
}
