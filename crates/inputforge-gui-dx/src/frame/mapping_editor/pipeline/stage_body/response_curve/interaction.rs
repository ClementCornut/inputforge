// Rust guideline compliant 2026-05-02

//! Pure pointer-event handlers for the F10 curve editor body.
//!
//! These fns take and return values, never Signals. The host
//! component projects Dioxus `PointerEvent` data to the primitives
//! consumed here (cursor pos in viewport coords, plot rect),
//! invokes a handler, then writes the resulting `BodyState'` and
//! optional `ResponseCurve'` back to its signals.

use inputforge_core::processing::curves::ResponseCurve;

use super::mutation;
use super::state::{BodyState, DragInProgress};

/// Bounding box of the SVG plot in viewport pixel coordinates. The
/// plot is square; `size` covers both width and height.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlotRect {
    pub x: f64,
    pub y: f64,
    pub size: f64,
}

/// Project a viewport pixel coordinate onto the plot's `viewBox`
/// coordinate system (`-1.05..1.05` square). The returned tuple is
/// engine-native `(input, output)`: the SVG y-flip is applied here
/// (output points up) so callers downstream see engine-native data.
#[must_use]
pub(crate) fn screen_to_viewbox(cursor: (f64, f64), r: &PlotRect) -> (f64, f64) {
    if r.size <= 0.0 {
        return (0.0, 0.0);
    }
    let nx = (cursor.0 - r.x) / r.size; // 0..1 left-to-right
    let ny = (cursor.1 - r.y) / r.size; // 0..1 top-to-bottom
    let input = -1.05 + nx * 2.1;
    // Top of plot is +output (1.05); bottom is -output (-1.05).
    let output = 1.05 - ny * 2.1;
    (input, output)
}

/// Project a viewBox `(input, output)` tuple onto viewport pixel
/// coordinates. Inverse of `screen_to_viewbox`.
fn viewbox_to_screen(p: (f64, f64), r: &PlotRect) -> (f64, f64) {
    let nx = (p.0 + 1.05) / 2.1;
    let ny = (1.05 - p.1) / 2.1;
    (r.x + nx * r.size, r.y + ny * r.size)
}

/// Find the anchor in `anchors` whose projected position is within
/// `radius_px` of `cursor`. Ties broken by lowest index.
#[must_use]
pub(crate) fn nearest_anchor(
    cursor: (f64, f64),
    anchors: &[(f64, f64)],
    r: &PlotRect,
    radius_px: f64,
) -> Option<usize> {
    let radius_sq = radius_px * radius_px;
    let mut best: Option<(usize, f64)> = None;
    for (i, a) in anchors.iter().enumerate() {
        let p = viewbox_to_screen(*a, r);
        let dx = p.0 - cursor.0;
        let dy = p.1 - cursor.1;
        let d2 = dx * dx + dy * dy;
        if d2 <= radius_sq {
            match best {
                Some((_, bd)) if bd <= d2 => {}
                _ => best = Some((i, d2)),
            }
        }
    }
    best.map(|(i, _)| i)
}

/// Hit-test radius in pixels for anchor selection and hover detection.
///
/// 10 px gives a comfortable tap target on both mouse and touch inputs
/// without making adjacent anchors ambiguous at typical curve densities.
pub(crate) const HIT_RADIUS_PX: f64 = 10.0;

/// Output of every handler except `handle_pointer_up`. `next_state` always
/// returns; `new_curve` is `Some` only when this event produced a new local
/// curve clone the host should adopt as its working copy; `changed` is `true`
/// when the caller must re-render or dispatch.
pub(crate) type HandlerOut = (BodyState, Option<ResponseCurve>, bool);

/// Handle a pointer-down event.
///
/// When the cursor is within [`HIT_RADIUS_PX`] of an anchor, starts a drag:
/// records `DragInProgress` in `state.dragging` and snapshots the current
/// curve into `state.pre_drag_curve`. Returns `(state', None, false)` in
/// all cases (the curve itself does not change on pointer-down; changes
/// only occur during pointer-move).
pub(crate) fn handle_pointer_down(
    mut state: BodyState,
    curve: &ResponseCurve,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    let Some(idx) = nearest_anchor(cursor, &state.cached_anchors, r, HIT_RADIUS_PX) else {
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

/// Handle a pointer-move event.
///
/// During a drag: projects `cursor` to viewBox coords, applies the drag to
/// a local clone of `curve`, sets `cache_dirty`, and returns the new curve
/// clone. Outside a drag: updates `hovered_point` only (no new curve).
pub(crate) fn handle_pointer_move(
    mut state: BodyState,
    curve: &ResponseCurve,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    if let Some(drag) = state.dragging.clone() {
        let p = screen_to_viewbox(cursor, r);
        let mut local = curve.clone();
        mutation::update_point_in_curve(&mut local, drag.point_index, p, drag.bounds);
        state.cache_dirty = true;
        return (state, Some(local), true);
    }
    state.hovered_point = nearest_anchor(cursor, &state.cached_anchors, r, HIT_RADIUS_PX);
    (state, None, false)
}

/// Pointer-up returns `Result<ResponseCurve, String>` so the host body can
/// write the validator's actual error to `EditorState.malformed_hints[stage_id]`.
///
/// Three return shapes:
/// - `(_, Ok(valid), true)`: drag committed; host dispatches `SetMapping`
///   with `valid` and pushes an undo entry. `next.pre_drag_curve` is `None`.
/// - `(_, Err(msg), false)` with `state.dragging.is_some()` on entry: drag
///   reverted by validator. `msg` carries the engine error text. `next.pre_drag_curve`
///   IS still populated; the host should `take()` it to restore the working
///   curve signal, and write `msg` to `EditorState.malformed_hints[stage_id]`.
/// - `(_, Err(""), false)` with `state.dragging.is_none()` on entry: no drag
///   was active; the host should treat this as a no-op (e.g. stray pointerup
///   from outside a drag). The empty error string is the sentinel; check
///   `state.dragging` before calling to avoid relying on it.
pub(crate) fn handle_pointer_up(
    mut state: BodyState,
    working_curve: &ResponseCurve,
) -> (BodyState, Result<ResponseCurve, String>, bool) {
    if state.dragging.is_none() {
        return (state, Err(String::new()), false);
    }
    state.dragging = None;
    state.cache_dirty = true;
    match mutation::reconstruct_curve(working_curve) {
        Ok(valid) => {
            state.pre_drag_curve = None;
            (state, Ok(valid), true)
        }
        Err(err) => {
            // Validation failed. `pre_drag_curve` remains populated in the returned
            // state so the host body can `take()` it and restore the working curve
            // signal. The host also writes `err` into
            // `EditorState.malformed_hints[stage_id]`.
            (state, Err(err), false)
        }
    }
}

/// Handle a double-click event.
///
/// Projects `cursor` to viewBox coords and inserts a new control point there
/// via [`mutation::add_control_point`]. Returns `(state', Some(new_curve), true)`
/// on success, or `(state, None, false)` when the click is outside the plot
/// bounds or the point cannot be inserted.
pub(crate) fn handle_double_click(
    mut state: BodyState,
    curve: &ResponseCurve,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    let p = screen_to_viewbox(cursor, r);
    // Bounds gate: a double-click outside the plot would otherwise be
    // clamped by `add_control_point` to a boundary anchor at (-1, ?) or
    // (1, ?), which is surprising UX.
    if !(-1.05..=1.05).contains(&p.0) || !(-1.05..=1.05).contains(&p.1) {
        return (state, None, false);
    }
    let mut local = curve.clone();
    if mutation::add_control_point(&mut local, p) {
        state.cache_dirty = true;
        return (state, Some(local), true);
    }
    (state, None, false)
}

/// Handle a context-menu event (right-click or long-press).
///
/// When `state.hovered_point` is `Some(idx)`, removes the control point at
/// `idx` via [`mutation::remove_control_point`] and clears `hovered_point`.
/// When no point is hovered, this is a no-op.
pub(crate) fn handle_context_menu(mut state: BodyState, curve: &ResponseCurve) -> HandlerOut {
    let Some(idx) = state.hovered_point else {
        return (state, None, false);
    };
    let mut local = curve.clone();
    if mutation::remove_control_point(&mut local, idx) {
        state.hovered_point = None;
        state.cache_dirty = true;
        return (state, Some(local), true);
    }
    (state, None, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[expect(
        redundant_imports,
        reason = "BodyState and DragInProgress re-enter via `super::*` but the \
                  explicit import keeps the tests self-documenting per the plan spec"
    )]
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::state::{
        BodyState, DragInProgress, extract_anchors,
    };
    #[expect(
        redundant_imports,
        reason = "ResponseCurve, BodyState, and DragInProgress re-enter via \
                  `super::*` but the explicit imports keep the tests self-documenting \
                  per the plan spec"
    )]
    use inputforge_core::processing::curves::ResponseCurve;

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
    fn screen_to_viewbox_maps_corners() {
        let r = rect();
        // top-left of plot maps to (-1.05, -1.05) in viewBox; in engine
        // coords (input axis horizontal) that's (input=-1.05, output=+1.05)
        // because SVG-y is flipped at render time.
        let p = screen_to_viewbox((10.0, 20.0), &r);
        assert!((p.0 - (-1.05)).abs() < 1e-6);
        assert!((p.1 - 1.05).abs() < 1e-6);
        // Center of plot maps to (0, 0).
        let p = screen_to_viewbox((10.0 + 120.0, 20.0 + 120.0), &r);
        assert!(p.0.abs() < 1e-6 && p.1.abs() < 1e-6);
    }

    #[test]
    fn nearest_anchor_within_radius() {
        let curve = seed_curve();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Center anchor (0, 0) projects to plot center.
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert_eq!(hit, Some(1));
    }

    #[test]
    fn nearest_anchor_outside_radius() {
        let curve = seed_curve();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Far from any anchor.
        let cursor = (10.0 + 120.0 + 60.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert!(hit.is_none());
    }

    #[test]
    fn nearest_anchor_at_bezier_junction_returns_lower_index() {
        // Two-segment bezier: anchors[3] (seg0.end) and anchors[4] (seg1.start)
        // coincide at (0, 0). A click at the junction must return the
        // lower index per `nearest_anchor`'s tie-breaking rule.
        let curve = inputforge_core::processing::curves::ResponseCurve::cubic_bezier(
            vec![
                inputforge_core::processing::curves::BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-0.5, -0.5),
                    control2: (-0.25, -0.25),
                    end: (0.0, 0.0),
                },
                inputforge_core::processing::curves::BezierSegment {
                    start: (0.0, 0.0),
                    control1: (0.25, 0.25),
                    control2: (0.5, 0.5),
                    end: (1.0, 1.0),
                },
            ],
            false,
        )
        .unwrap();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Junction (0, 0) projects to plot center.
        let cursor = (10.0 + 120.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert_eq!(
            hit,
            Some(3),
            "junction tie must return lower index (3, not 4)"
        );
    }

    #[test]
    fn pointer_down_on_anchor_starts_drag_and_snapshots() {
        let curve = seed_curve();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            ..BodyState::default()
        };
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (next, _new_curve, _changed) = handle_pointer_down(state, &curve, cursor, &rect());
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
        let (next, new_curve, changed) = handle_pointer_down(state, &curve, cursor, &rect());
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
        let (state, _, _) = handle_pointer_down(state, &curve, cursor_down, &rect());
        // Move down-and-right.
        let cursor_move = (10.0 + 120.0 + 30.0, 20.0 + 120.0 + 10.0);
        let (next, new_curve, changed) = handle_pointer_move(state, &curve, cursor_move, &rect());
        assert!(changed);
        let new_curve = new_curve.expect("drag-move yields a new local curve");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            // Center moved away from origin.
            assert!(points[1].0 != 0.0 || points[1].1 != 0.0);
        }
        assert!(next.cache_dirty);
    }

    #[test]
    fn pointer_move_idle_updates_hover_only() {
        let curve = seed_curve();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            ..BodyState::default()
        };
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (next, new_curve, changed) = handle_pointer_move(state, &curve, cursor, &rect());
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
            handle_double_click(BodyState::default(), &curve, cursor, &rect());
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
        let (state, _, _) = handle_pointer_down(state, &curve, down, &rect());
        let mv = (10.0 + 120.0 + 24.0, 20.0 + 120.0); // +20px right, same y
        let (_next, new_curve, _) = handle_pointer_move(state, &curve, mv, &rect());
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
