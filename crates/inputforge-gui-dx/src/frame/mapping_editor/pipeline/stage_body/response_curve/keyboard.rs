// Rust guideline compliant 2026-05-02

//! Pure keyboard handler for the F10 curve-editor body.

use inputforge_core::processing::curves::ResponseCurve;

use super::mutation;
use super::state::BodyState;

/// Small nudge step applied by a plain arrow-key press.
///
/// 0.01 corresponds to 1% of the full `[-1, 1]` range per keypress, giving
/// fine-grained positioning without requiring the mouse.
const KEY_NUDGE_STEP: f64 = 0.01;

/// Large nudge step applied when Shift is held while pressing an arrow key.
///
/// 0.10 is 10x the small step; matches the conventional "coarse move"
/// semantics in accessibility and creative software.
const KEY_NUDGE_STEP_LARGE: f64 = 0.10;

/// Inputs the host normalizes from a Dioxus `KeyboardEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyInput {
    Tab,
    ShiftTab,
    ArrowLeft { shift: bool },
    ArrowRight { shift: bool },
    ArrowUp { shift: bool },
    ArrowDown { shift: bool },
    Home,
    End,
    Enter,
    Delete,
    Escape,
}

/// Coarse-grained kind used by the coalesce-window state. Two presses
/// of the same `KeyKind` within
/// `instruments::nudge_coalesce::COALESCE_WINDOW_MS` merge into a
/// single undo entry; presses of different kinds always push.
///
/// The `Arrow` prefix is kept verbatim because it mirrors the `KeyInput`
/// variant names and the plan's literal source; the enum is small and
/// self-contained so the repeated prefix does not cause confusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::enum_variant_names,
    reason = "Arrow prefix mirrors KeyInput names and plan spec; \
              the four-variant set is self-contained and unambiguous"
)]
pub(crate) enum KeyKind {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
}

impl KeyInput {
    fn nudge_kind(self) -> Option<KeyKind> {
        Some(match self {
            Self::ArrowLeft { .. } => KeyKind::ArrowLeft,
            Self::ArrowRight { .. } => KeyKind::ArrowRight,
            Self::ArrowUp { .. } => KeyKind::ArrowUp,
            Self::ArrowDown { .. } => KeyKind::ArrowDown,
            _ => return None,
        })
    }
}

/// Tells the host how to record this key event in the undo log.
///
/// The `UndoLog` API at `mapping_editor/undo_log.rs:95-113` exposes only
/// `push_edit(key, before, kind, label)`; there is no `merge_with_top` /
/// `update_top` operation. `MergeUndo` therefore relies on the following
/// host-side contract:
///
/// 1. On the FIRST key in a coalesce streak (`PushUndo`), the host
///    captures `mapping_before = mapping_at(actions_root, mapping_key)`
///    and calls `dispatch_stage_edit(...)`, which internally calls
///    `undo_log.push_edit(key, mapping_before, StageEdit, label)`.
/// 2. On every SUBSEQUENT key in the same streak (`MergeUndo`), the host
///    calls `dispatch_stage_edit_no_undo(...)`, which dispatches
///    `EngineCommand::SetMapping` to the engine but does NOT touch the
///    undo log. The first entry's `mapping_before` already captures the
///    pre-streak state, so undo restores correctly.
/// 3. Redo replays the first nudge's `SetMapping` only (not the streak
///    total). Accepted as a deliberate UX simplification.
///
/// The 250 ms coalesce window is owned by `handle_key`; the host does not
/// need to track timing. The host MUST treat `MergeUndo` as "skip the
/// undo write but still dispatch the engine command".
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeyOutcome {
    PushUndo { label: String },
    MergeUndo,
}

/// Return type of [`handle_key`]: `(next_state, new_curve, undo_outcome, changed)`.
pub(crate) type KeyHandlerOut = (BodyState, Option<ResponseCurve>, Option<KeyOutcome>, bool);

/// Handle one keyboard event and return the next body state.
///
/// The function is pure: it takes ownership of `state`, applies the key
/// semantics, and returns the updated state together with an optional new
/// curve and undo hint. No Signals are touched.
///
/// `now_ms` is the current timestamp in milliseconds since component mount
/// (or any monotonic epoch); it drives the 250 ms same-key coalesce window.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the function is a single dispatch table; extracting sub-handlers \
              would obscure the linear flow without reducing actual complexity"
)]
pub(crate) fn handle_key(
    mut state: BodyState,
    curve: &ResponseCurve,
    key: KeyInput,
    now_ms: u64,
) -> KeyHandlerOut {
    if let KeyInput::Escape = key {
        if state.dragging.is_some() {
            let revert = state.pre_drag_curve.take();
            state.dragging = None;
            state.cache_dirty = true;
            return (state, revert, None, false);
        }
        return (state, None, None, false);
    }

    match key {
        KeyInput::Tab | KeyInput::ShiftTab => {
            let new_focus = advance_focus(
                curve,
                &state.cached_anchors,
                state.focused_point,
                matches!(key, KeyInput::ShiftTab),
            );
            state.focused_point = new_focus;
            return (state, None, None, false);
        }
        KeyInput::Home => {
            state.focused_point = if state.cached_anchors.is_empty() {
                None
            } else {
                Some(0)
            };
            return (state, None, None, false);
        }
        KeyInput::End => {
            state.focused_point = state.cached_anchors.len().checked_sub(1);
            return (state, None, None, false);
        }
        _ => {}
    }

    let Some(idx) = state.focused_point else {
        return (state, None, None, false);
    };

    let outcome_label_for_nudge = "curve: nudge".to_owned();

    match key {
        KeyInput::ArrowLeft { shift }
        | KeyInput::ArrowRight { shift }
        | KeyInput::ArrowUp { shift }
        | KeyInput::ArrowDown { shift } => {
            let step = if shift {
                KEY_NUDGE_STEP_LARGE
            } else {
                KEY_NUDGE_STEP
            };
            let (dx, dy) = match key {
                KeyInput::ArrowLeft { .. } => (-step, 0.0),
                KeyInput::ArrowRight { .. } => (step, 0.0),
                KeyInput::ArrowUp { .. } => (0.0, step),
                KeyInput::ArrowDown { .. } => (0.0, -step),
                _ => unreachable!(),
            };
            let cur = *state.cached_anchors.get(idx).unwrap_or(&(0.0, 0.0));
            let bounds = mutation::adjacent_x_bounds(curve, idx);
            let new_pos = (cur.0 + dx, cur.1 + dy);
            let mut local = curve.clone();
            mutation::update_point_in_curve(&mut local, idx, new_pos, bounds);
            // Conflict 2 resolution: reconstruct_curve returns Result, not Option.
            let Ok(valid) = mutation::reconstruct_curve(&local) else {
                return (state, None, None, false);
            };
            let kind = key.nudge_kind().expect("arrow key has nudge kind");
            let merge = state.nudge_coalesce.should_merge(now_ms, kind);
            state.nudge_coalesce.record(now_ms, kind);
            state.cache_dirty = true;
            let outcome = if merge {
                KeyOutcome::MergeUndo
            } else {
                KeyOutcome::PushUndo {
                    label: outcome_label_for_nudge,
                }
            };
            (state, Some(valid), Some(outcome), true)
        }
        KeyInput::Enter => {
            // Insert at midpoint between idx and its right neighbor.
            // No-op for rightmost anchor or bezier handle.
            let local_idx = idx;
            let anchor = match state.cached_anchors.get(local_idx) {
                Some(a) => *a,
                None => return (state, None, None, false),
            };
            let next = match state.cached_anchors.get(local_idx + 1) {
                Some(n) => *n,
                None => return (state, None, None, false),
            };
            // Bezier handle filter: only insert when both points are anchors
            // (local % 4 in {0, 3} for bezier; piecewise/spline always pass).
            if !is_anchor_index(curve, local_idx) || !is_anchor_index(curve, local_idx + 1) {
                return (state, None, None, false);
            }
            let mid = (
                f64::midpoint(anchor.0, next.0),
                f64::midpoint(anchor.1, next.1),
            );
            let mut local = curve.clone();
            if !mutation::add_control_point(&mut local, mid) {
                return (state, None, None, false);
            }
            state.cache_dirty = true;
            state.nudge_coalesce.reset();
            (
                state,
                Some(local),
                Some(KeyOutcome::PushUndo {
                    label: format!("curve: add point at ({:.2}, {:.2})", mid.0, mid.1),
                }),
                true,
            )
        }
        KeyInput::Delete => {
            let mut local = curve.clone();
            if !mutation::remove_control_point(&mut local, idx) {
                return (state, None, None, false);
            }
            state.cache_dirty = true;
            state.nudge_coalesce.reset();
            // Clamp focused index after removal.
            let new_anchors = super::state::extract_anchors(&local);
            state.focused_point = if new_anchors.is_empty() {
                None
            } else {
                Some(idx.min(new_anchors.len() - 1))
            };
            (
                state,
                Some(local),
                Some(KeyOutcome::PushUndo {
                    label: "curve: remove point".to_owned(),
                }),
                true,
            )
        }
        KeyInput::Tab | KeyInput::ShiftTab | KeyInput::Home | KeyInput::End | KeyInput::Escape => {
            unreachable!("handled above")
        }
    }
}

fn is_anchor_index(curve: &ResponseCurve, idx: usize) -> bool {
    match curve {
        ResponseCurve::PiecewiseLinear { .. } | ResponseCurve::CubicSpline { .. } => true,
        ResponseCurve::CubicBezier { .. } => matches!(idx % 4, 0 | 3),
    }
}

fn advance_focus(
    curve: &ResponseCurve,
    anchors: &[(f64, f64)],
    current: Option<usize>,
    backward: bool,
) -> Option<usize> {
    if anchors.is_empty() {
        return None;
    }
    let len = anchors.len();
    // Short-circuit: ShiftTab from index 0 (or from no focus) releases focus
    // to the browser rather than trapping the user at the first anchor.
    if backward && current.is_none_or(|i| i == 0) {
        return None;
    }
    let start = current.map_or(0, |i| if backward { i.saturating_sub(1) } else { i + 1 });
    let order: Vec<usize> = if backward {
        (0..len).rev().collect()
    } else {
        (0..len).collect()
    };
    // Filter: skip duplicate-junction points (segN.end == seg(N+1).start).
    let visit_filter = |i: usize| -> bool {
        if let ResponseCurve::CubicBezier { .. } = curve
            && i.is_multiple_of(4)
            && i > 0
        {
            // seg.start where i = 4*k, k > 0; coincides with prior seg.end.
            if let Some(prev) = anchors.get(i.saturating_sub(1)).copied()
                && let Some(here) = anchors.get(i).copied()
                && (prev.0 - here.0).abs() < f64::EPSILON
                && (prev.1 - here.1).abs() < f64::EPSILON
            {
                return false;
            }
        }
        true
    };
    // Scan in order from `start`, skipping filtered indices.
    order
        .iter()
        .skip_while(|&&i| if backward { i > start } else { i < start })
        .find(|&&i| visit_filter(i))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::state::{
        DragInProgress, extract_anchors,
    };
    #[expect(
        redundant_imports,
        reason = "BezierSegment and ResponseCurve are re-imported for test \
                  self-documentation; the explicit path mirrors the plan spec literal block"
    )]
    use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

    fn seed() -> (ResponseCurve, BodyState) {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            focused_point: Some(1),
            ..BodyState::default()
        };
        (curve, state)
    }

    #[test]
    fn tab_advances_focus_no_wrap() {
        // F10 keyboard does NOT wrap at the end of the anchor list; Tab
        // returns None so the browser advances focus past the plot.
        let (curve, state) = seed();
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::Tab, 0);
        assert_eq!(next.focused_point, Some(2));
    }

    #[test]
    fn tab_skips_duplicate_bezier_endpoints() {
        // Two-segment bezier: 8 anchors, but seg2.start coincides with seg1.end.
        let curve = ResponseCurve::cubic_bezier(
            vec![
                BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-0.5, -0.5),
                    control2: (-0.25, -0.25),
                    end: (0.0, 0.0),
                },
                BezierSegment {
                    start: (0.0, 0.0),
                    control1: (0.25, 0.25),
                    control2: (0.5, 0.5),
                    end: (1.0, 1.0),
                },
            ],
            false,
        )
        .unwrap();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            focused_point: Some(3), // seg1.end (0,0)
            ..BodyState::default()
        };
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::Tab, 0);
        // Next visit must skip index 4 (seg2.start, same point) and
        // land on index 5 (seg2.control1).
        assert_eq!(next.focused_point, Some(5));
    }

    #[test]
    fn shift_tab_skips_duplicate_bezier_endpoints_backward() {
        // Same 2-segment bezier; backward navigation from seg2.control1 (idx 5)
        // must skip the duplicate junction at idx 4 and land on idx 3 (seg1.end).
        let curve = ResponseCurve::cubic_bezier(
            vec![
                BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-0.5, -0.5),
                    control2: (-0.25, -0.25),
                    end: (0.0, 0.0),
                },
                BezierSegment {
                    start: (0.0, 0.0),
                    control1: (0.25, 0.25),
                    control2: (0.5, 0.5),
                    end: (1.0, 1.0),
                },
            ],
            false,
        )
        .unwrap();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            focused_point: Some(5),
            ..BodyState::default()
        };
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::ShiftTab, 0);
        assert_eq!(
            next.focused_point,
            Some(3),
            "ShiftTab from 5 must skip junction at 4 and land on 3",
        );
    }

    #[test]
    fn enter_on_bezier_anchor_with_handle_neighbor_is_no_op() {
        // 1-segment bezier: idx 0 is anchor (seg.start), idx 1 is handle
        // (seg.control1). Enter on idx 0 should NOT insert because the
        // right neighbor (idx 1) is not an anchor.
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
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            focused_point: Some(0),
            ..BodyState::default()
        };
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Enter, 0);
        assert!(
            !changed,
            "Enter must be a no-op when right neighbor is a handle"
        );
        assert!(new_curve.is_none());
    }

    #[test]
    fn arrow_right_nudges_x_by_step() {
        let (curve, state) = seed();
        let (_next, new_curve, outcome, changed) =
            handle_key(state, &curve, KeyInput::ArrowRight { shift: false }, 1000);
        assert!(changed);
        let new_curve = new_curve.expect("nudge yields a curve");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            assert!((points[1].0 - 0.01).abs() < 1e-9);
        }
        assert!(matches!(outcome, Some(KeyOutcome::PushUndo { .. })));
    }

    #[test]
    fn shift_arrow_uses_large_step() {
        let (curve, state) = seed();
        let (_, new_curve, _, _) =
            handle_key(state, &curve, KeyInput::ArrowRight { shift: true }, 1000);
        if let Some(ResponseCurve::PiecewiseLinear { points, .. }) = new_curve {
            assert!((points[1].0 - 0.10).abs() < 1e-9);
        }
    }

    #[test]
    fn enter_inserts_midpoint_when_focused_anchor_has_right_neighbor() {
        let (curve, state) = seed();
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Enter, 1000);
        assert!(changed);
        let new_curve = new_curve.expect("Enter inserts");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            assert_eq!(points.len(), 4);
        }
    }

    #[test]
    fn enter_on_rightmost_anchor_is_no_op() {
        let (curve, mut state) = seed();
        state.focused_point = Some(2);
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Enter, 1000);
        assert!(!changed);
        assert!(new_curve.is_none());
    }

    #[test]
    fn delete_center_anchor_succeeds() {
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (-0.4, -0.4), (0.4, 0.4), (1.0, 1.0)],
            false,
        )
        .unwrap();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            focused_point: Some(1),
            ..BodyState::default()
        };
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Delete, 1000);
        assert!(changed);
        if let Some(ResponseCurve::PiecewiseLinear { points, .. }) = new_curve {
            assert_eq!(points.len(), 3);
        }
    }

    #[test]
    fn delete_edge_is_no_op() {
        let (curve, mut state) = seed();
        state.focused_point = Some(0);
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Delete, 1000);
        assert!(!changed);
        assert!(new_curve.is_none());
    }

    #[test]
    fn escape_during_drag_reverts() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.5, 0.5), (1.0, 1.0)], false)
                .unwrap();
        let pre =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let state = BodyState {
            cached_anchors: extract_anchors(&curve),
            dragging: Some(DragInProgress {
                point_index: 1,
                bounds: (-1.0, 1.0),
            }),
            pre_drag_curve: Some(pre.clone()),
            ..BodyState::default()
        };
        let (next, new_curve, _, _) = handle_key(state, &curve, KeyInput::Escape, 1000);
        assert!(next.dragging.is_none());
        let reverted = new_curve.expect("Escape during drag reverts");
        if let ResponseCurve::PiecewiseLinear { points, .. } = reverted {
            assert!(points[1].0.abs() < 1e-9);
        }
    }

    #[test]
    fn home_and_end_jump_focus() {
        let (curve, state) = seed();
        let (next, _, _, _) = handle_key(state.clone(), &curve, KeyInput::Home, 1000);
        assert_eq!(next.focused_point, Some(0));
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::End, 1000);
        assert_eq!(next.focused_point, Some(2));
    }

    #[test]
    fn same_key_within_window_merges_undo() {
        let (curve, mut state) = seed();
        state.nudge_coalesce.record(1000, KeyKind::ArrowRight);
        let (_, _, outcome, _) =
            handle_key(state, &curve, KeyInput::ArrowRight { shift: false }, 1100);
        match outcome {
            Some(KeyOutcome::MergeUndo) => {}
            other => panic!("expected MergeUndo, got {other:?}"),
        }
    }

    #[test]
    fn same_key_after_window_pushes_new_undo() {
        let (curve, mut state) = seed();
        state.nudge_coalesce.record(1000, KeyKind::ArrowRight);
        let (_, _, outcome, _) =
            handle_key(state, &curve, KeyInput::ArrowRight { shift: false }, 1500);
        assert!(matches!(outcome, Some(KeyOutcome::PushUndo { .. })));
    }

    #[test]
    fn shift_tab_from_first_anchor_releases_focus() {
        // Mirror of `tab_advances_focus_no_wrap` for the backward direction:
        // ShiftTab on the leftmost anchor must return None so the browser
        // advances focus past the plot. Without this guard the user is
        // trapped at index 0.
        let (curve, mut state) = seed();
        state.focused_point = Some(0);
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::ShiftTab, 0);
        assert_eq!(
            next.focused_point, None,
            "ShiftTab on first anchor releases focus"
        );
    }
}
