//! Curve keyboard edits and focus observations.
use super::focus::{advance_focus, is_anchor_index};
use inputforge_core::processing::curves::ResponseCurve;

use super::mutation;
use super::state::BodyState;

// Plain and Shift arrows preserve the existing 1% and 10% editing steps.
const KEY_NUDGE_STEP: f64 = 0.01;

const KEY_NUDGE_STEP_LARGE: f64 = 0.10;

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
    pub(crate) fn nudge_kind(self) -> Option<KeyKind> {
        Some(match self {
            Self::ArrowLeft { .. } => KeyKind::ArrowLeft,
            Self::ArrowRight { .. } => KeyKind::ArrowRight,
            Self::ArrowUp { .. } => KeyKind::ArrowUp,
            Self::ArrowDown { .. } => KeyKind::ArrowDown,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeyOutcome {
    PushUndo { label: String },
}

pub(crate) type KeyHandlerOut = (BodyState, Option<ResponseCurve>, Option<KeyOutcome>, bool);

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
) -> KeyHandlerOut {
    if let KeyInput::Escape = key {
        if state.dragging.is_some() {
            let revert = state.pre_drag_curve.take();
            state.dragging = None;
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
            let Ok(valid) = mutation::reconstruct_curve(&local) else {
                return (state, None, None, false);
            };
            let outcome = KeyOutcome::PushUndo {
                label: outcome_label_for_nudge,
            };
            (state, Some(valid), Some(outcome), true)
        }
        KeyInput::Enter => {
            let local_idx = idx;
            let anchor = match state.cached_anchors.get(local_idx) {
                Some(a) => *a,
                None => return (state, None, None, false),
            };
            let next = match state.cached_anchors.get(local_idx + 1) {
                Some(n) => *n,
                None => return (state, None, None, false),
            };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping_edits::instruments::response_curve::state::{
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
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::Tab);
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
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::Tab);
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
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::ShiftTab);
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
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Enter);
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
            handle_key(state, &curve, KeyInput::ArrowRight { shift: false });
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
        let (_, new_curve, _, _) = handle_key(state, &curve, KeyInput::ArrowRight { shift: true });
        if let Some(ResponseCurve::PiecewiseLinear { points, .. }) = new_curve {
            assert!((points[1].0 - 0.10).abs() < 1e-9);
        }
    }

    #[test]
    fn enter_inserts_midpoint_when_focused_anchor_has_right_neighbor() {
        let (curve, state) = seed();
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Enter);
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
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Enter);
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
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Delete);
        assert!(changed);
        if let Some(ResponseCurve::PiecewiseLinear { points, .. }) = new_curve {
            assert_eq!(points.len(), 3);
        }
    }

    #[test]
    fn delete_edge_is_no_op() {
        let (curve, mut state) = seed();
        state.focused_point = Some(0);
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Delete);
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
        let (next, new_curve, _, _) = handle_key(state, &curve, KeyInput::Escape);
        assert!(next.dragging.is_none());
        let reverted = new_curve.expect("Escape during drag reverts");
        if let ResponseCurve::PiecewiseLinear { points, .. } = reverted {
            assert!(points[1].0.abs() < 1e-9);
        }
    }

    #[test]
    fn home_and_end_jump_focus() {
        let (curve, state) = seed();
        let (next, _, _, _) = handle_key(state.clone(), &curve, KeyInput::Home);
        assert_eq!(next.focused_point, Some(0));
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::End);
        assert_eq!(next.focused_point, Some(2));
    }

    #[test]
    fn shift_tab_from_first_anchor_releases_focus() {
        // Mirror of `tab_advances_focus_no_wrap` for the backward direction:
        // ShiftTab on the leftmost anchor must return None so the browser
        // advances focus past the plot. Without this guard the user is
        // trapped at index 0.
        let (curve, mut state) = seed();
        state.focused_point = Some(0);
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::ShiftTab);
        assert_eq!(
            next.focused_point, None,
            "ShiftTab on first anchor releases focus"
        );
    }
}
