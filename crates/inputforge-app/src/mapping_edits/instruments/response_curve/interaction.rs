//! Response Curve interaction behavior.
use super::mutation;
use super::state::{BodyState, DragInProgress};
use inputforge_core::processing::curves::ResponseCurve;
use inputforge_gui_dx::instruments::Observation;
pub(crate) type HandlerOut = (BodyState, Option<ResponseCurve>, bool);
pub(crate) fn handle_pointer_down(
    mut state: BodyState,
    curve: &ResponseCurve,
    observation: &Observation,
) -> HandlerOut {
    let Some(idx) = observation.hit.filter(|&i| i < state.cached_anchors.len()) else {
        return (state, None, false);
    };
    let bounds = mutation::adjacent_x_bounds(curve, idx);
    state.dragging = Some(DragInProgress {
        point_index: idx,
        bounds,
    });
    state.pre_drag_curve = Some(curve.clone());
    (state, None, false)
}

pub(crate) fn handle_pointer_move(
    mut state: BodyState,
    curve: &ResponseCurve,
    observation: &Observation,
) -> HandlerOut {
    if let Some(drag) = state.dragging.clone() {
        let Some(p) = observation.position else {
            return (state, None, false);
        };
        let mut local = curve.clone();
        mutation::update_point_in_curve(&mut local, drag.point_index, p, drag.bounds);
        return (state, Some(local), true);
    }
    state.hovered_point = observation.hit.filter(|&i| i < state.cached_anchors.len());
    (state, None, false)
}

pub(crate) fn handle_pointer_up(
    mut state: BodyState,
    working_curve: &ResponseCurve,
) -> (BodyState, Result<ResponseCurve, String>, bool) {
    if state.dragging.is_none() {
        return (state, Err(String::new()), false);
    }
    state.dragging = None;
    match mutation::reconstruct_curve(working_curve) {
        Ok(valid) => {
            state.pre_drag_curve = None;
            (state, Ok(valid), true)
        }
        Err(err) => (state, Err(err), false),
    }
}

pub(crate) fn handle_double_click(
    state: BodyState,
    curve: &ResponseCurve,
    observation: &Observation,
) -> HandlerOut {
    let Some(p) = observation.position else {
        return (state, None, false);
    };
    if !(-1.05..=1.05).contains(&p.0) || !(-1.05..=1.05).contains(&p.1) {
        return (state, None, false);
    }
    let mut local = curve.clone();
    if mutation::add_control_point(&mut local, p) {
        return (state, Some(local), true);
    }
    (state, None, false)
}

pub(crate) fn handle_context_menu(mut state: BodyState, curve: &ResponseCurve) -> HandlerOut {
    let Some(idx) = state.hovered_point else {
        return (state, None, false);
    };
    let mut local = curve.clone();
    if mutation::remove_control_point(&mut local, idx) {
        state.hovered_point = None;
        return (state, Some(local), true);
    }
    (state, None, false)
}

#[cfg(test)]
mod tests {
    use super::super::super::pointer_fixture::*;
    use super::*;
    #[expect(
        redundant_imports,
        reason = "BodyState and DragInProgress re-enter via `super::*` but the \
                  explicit import keeps the tests self-documenting per the plan spec"
    )]
    use crate::mapping_edits::instruments::response_curve::state::{
        BodyState, DragInProgress, extract_anchors,
    };
    #[expect(
        redundant_imports,
        reason = "ResponseCurve, BodyState, and DragInProgress re-enter via \
                  `super::*` but the explicit imports keep the tests self-documenting \
                  per the plan spec"
    )]
    use inputforge_core::processing::curves::ResponseCurve;

    fn observe(cursor: (f64, f64), r: &PlotRect, points: &[(f64, f64)]) -> Observation {
        let hit = nearest_anchor(cursor, points, r, 10.0);
        let centers = points.len() == 4 && [1, 2].iter().all(|&i| near(cursor, points[i], r, 10.0));
        let split_dx = points
            .get(1)
            .map_or(0.0, |p| cursor.0 - viewbox_to_screen(*p, r).0);
        Observation {
            position: screen_to_viewbox(cursor, r),
            hit,
            centers,
            split_dx,
        }
    }
    fn handle_pointer_down_pixels(
        state: BodyState,
        value: &ResponseCurve,
        cursor: (f64, f64),
        r: &PlotRect,
    ) -> HandlerOut {
        handle_pointer_down(state, value, &observe(cursor, r, &extract_anchors(value)))
    }
    fn handle_pointer_move_pixels(
        state: BodyState,
        value: &ResponseCurve,
        cursor: (f64, f64),
        r: &PlotRect,
    ) -> HandlerOut {
        handle_pointer_move(state, value, &observe(cursor, r, &extract_anchors(value)))
    }
    fn handle_double_click_pixels(
        state: BodyState,
        value: &ResponseCurve,
        cursor: (f64, f64),
        r: &PlotRect,
    ) -> HandlerOut {
        handle_double_click(state, value, &observe(cursor, r, &extract_anchors(value)))
    }

    fn seed_curve() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
    }

    fn rect() -> PlotRect {
        // Square plot, 240px, top-left at (10, 20).
        PlotRect {
            x: 10.0,
            y: 20.0,
            size: 240.0,
        }
    }

    #[test]
    fn pointer_down_on_anchor_starts_drag_and_snapshots() {
        let curve = seed_curve();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            ..BodyState::default()
        };
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (next, _new_curve, _changed) =
            handle_pointer_down_pixels(state, &curve, cursor, &rect());
        assert!(next.dragging.is_some());
        assert_eq!(next.dragging.as_ref().unwrap().point_index, 1);
        assert!(next.pre_drag_curve.is_some());
    }

    #[test]
    fn pointer_down_miss_no_drag() {
        let curve = seed_curve();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            ..BodyState::default()
        };
        let cursor = (10.0 + 120.0 + 60.0, 20.0 + 120.0);
        let (next, new_curve, changed) = handle_pointer_down_pixels(state, &curve, cursor, &rect());
        assert!(next.dragging.is_none());
        assert!(new_curve.is_none());
        assert!(!changed);
    }

    #[test]
    fn pointer_move_during_drag_updates_curve_locally() {
        let curve = seed_curve();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            ..BodyState::default()
        };
        // Simulate drag start.
        let cursor_down = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (state, _, _) = handle_pointer_down_pixels(state, &curve, cursor_down, &rect());
        // Move down-and-right.
        let cursor_move = (10.0 + 120.0 + 30.0, 20.0 + 120.0 + 10.0);
        let (_next, new_curve, changed) =
            handle_pointer_move_pixels(state, &curve, cursor_move, &rect());
        assert!(changed);
        let new_curve = new_curve.expect("drag-move yields a new local curve");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            // Center moved away from origin.
            assert!(points[1].0 != 0.0 || points[1].1 != 0.0);
        }
    }

    #[test]
    fn pointer_move_idle_updates_hover_only() {
        let curve = seed_curve();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            ..BodyState::default()
        };
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (next, new_curve, changed) = handle_pointer_move_pixels(state, &curve, cursor, &rect());
        assert!(!changed);
        assert!(new_curve.is_none());
        assert_eq!(next.hovered_point, Some(1));
    }

    #[test]
    fn pointer_up_after_drag_validates_and_commits() {
        let curve = seed_curve();
        // Pretend we've already mid-dragged the curve into a valid state.
        let dragged = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.1, 0.2), (1.0, 1.0)],
            symmetric: false,
        };
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            dragging: Some(DragInProgress {
                point_index: 1,
                bounds: (-1.0, 1.0),
            }),
            pre_drag_curve: Some(curve.clone()),
            ..BodyState::default()
        };
        let (next, committed, _) = handle_pointer_up(state, &dragged);
        assert!(next.dragging.is_none());
        assert!(next.pre_drag_curve.is_none());
        let committed = committed.expect("valid drag commits");
        if let ResponseCurve::PiecewiseLinear { points, .. } = committed {
            assert!((points[1].0 - 0.1).abs() < 1e-9);
        }
    }

    #[test]
    fn pointer_up_after_invalid_drag_reverts() {
        let curve = seed_curve();
        // An invalid mid-drag state: x values not strictly increasing.
        let dragged = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (1.0, 0.2), (1.0, 1.0)],
            symmetric: false,
        };
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            dragging: Some(DragInProgress {
                point_index: 1,
                bounds: (-1.0, 1.0),
            }),
            pre_drag_curve: Some(curve.clone()),
            ..BodyState::default()
        };
        let (next, committed, _) = handle_pointer_up(state, &dragged);
        assert!(next.dragging.is_none());
        // `committed` is `Result<ResponseCurve, String>`. On invalid curves
        // the handler returns Err with the validator's actual message; the
        // host body writes this to EditorState.malformed_hints[stage_id].
        // The handler does NOT carry per-body validator state.
        let err = committed.expect_err("invalid curve must not commit");
        assert!(!err.is_empty(), "validator error string surfaces");
    }

    #[test]
    fn double_click_adds_point_when_valid() {
        let curve = seed_curve();
        let cursor = (10.0 + 60.0, 20.0 + 80.0); // somewhere inside the plot
        let (_next, new_curve, changed) =
            handle_double_click_pixels(BodyState::default(), &curve, cursor, &rect());
        assert!(changed);
        assert!(new_curve.is_some());
    }

    #[test]
    fn context_menu_with_hover_removes_point() {
        // Multi-anchor curve so removal is allowed.
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (-0.3, -0.3), (0.3, 0.3), (1.0, 1.0)],
            false,
        )
        .unwrap();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            hovered_point: Some(1),
            ..BodyState::default()
        };
        let (next, new_curve, changed) = handle_context_menu(state, &curve);
        assert!(changed);
        let new_curve = new_curve.expect("removable hovered point yields a new curve");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            assert_eq!(points.len(), 3);
        }
        assert!(next.hovered_point.is_none(), "hover clears after remove");
    }

    #[test]
    fn context_menu_without_hover_is_no_op() {
        let curve = seed_curve();
        let (_next, new_curve, changed) = handle_context_menu(BodyState::default(), &curve);
        assert!(!changed);
        assert!(new_curve.is_none());
    }

    #[test]
    fn interaction_uses_engine_native_coordinates() {
        // Regression: dragging the center anchor right by a known amount
        // produces a curve whose middle point's x increased, NOT y. The
        // SVG port plots input on X and output on Y, so engine-native
        // (input, output) tuples flow through unchanged. The egui code
        // plotted output on X and input on Y (see egui-ref interaction.rs:73-74,
        // 99-100 for `PlotPoint::new(visual_pos.y, visual_pos.x)`); that swap
        // was correct for the egui visual axes and is NOT a defect that this
        // test guards against. This test guards against accidentally porting
        // that visual-axis-swap logic into the SVG layer where it does not
        // belong.
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            ..BodyState::default()
        };
        let down = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (state, _, _) = handle_pointer_down_pixels(state, &curve, down, &rect());
        let mv = (10.0 + 120.0 + 24.0, 20.0 + 120.0); // +20px right, same y
        let (_next, new_curve, _) = handle_pointer_move_pixels(state, &curve, mv, &rect());
        if let Some(ResponseCurve::PiecewiseLinear { points, .. }) = new_curve {
            // x should have increased; y should be ~0.
            assert!(
                points[1].0 > 0.05,
                "x must have moved right, got {}",
                points[1].0
            );
            assert!(
                points[1].1.abs() < 0.05,
                "y must stay ~0, got {}",
                points[1].1
            );
        }
    }

    #[test]
    fn pointer_up_invalid_keeps_pre_drag_curve_for_host_revert() {
        let curve = seed_curve();
        let invalid = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (1.0, 0.2), (1.0, 1.0)],
            symmetric: false,
        };
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            dragging: Some(DragInProgress {
                point_index: 1,
                bounds: (-1.0, 1.0),
            }),
            pre_drag_curve: Some(curve.clone()),
            ..BodyState::default()
        };
        let (next, committed, _) = handle_pointer_up(state, &invalid);
        assert!(committed.is_err(), "invalid curve must produce Err");
        assert!(
            next.pre_drag_curve.is_some(),
            "pre_drag_curve must survive on Err so host can restore"
        );
    }
}
