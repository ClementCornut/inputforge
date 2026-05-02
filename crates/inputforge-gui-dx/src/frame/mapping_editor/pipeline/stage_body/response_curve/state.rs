//! `BodyState` and helpers used by the F10 curve-editor body.
// Rust guideline compliant 2026-05-02

use inputforge_core::processing::curves::ResponseCurve;

use super::keyboard::KeyKind;

/// Per-mounted-component state held in a `Signal<BodyState>` inside
/// `ResponseCurveBody`. Pure data; no Signals.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BodyState {
    pub dragging: Option<DragInProgress>,
    pub hovered_point: Option<usize>,
    /// Keyboard-focused anchor; intentionally separate from `hovered_point`.
    pub focused_point: Option<usize>,
    /// Snapshot taken at drag start, used to revert on validation failure.
    pub pre_drag_curve: Option<ResponseCurve>,
    /// 200-sample polyline; engine-native (input, output).
    pub cached_path: Vec<(f64, f64)>,
    /// Flat list of draggable points; mutation.rs index space.
    pub cached_anchors: Vec<(f64, f64)>,
    pub cache_dirty: bool,
    /// Timestamp (ms since component mount) of the last keyboard nudge.
    /// Drives Task 7's 250 ms same-key coalesce window for undo merging.
    pub last_nudge_at_ms: Option<u64>,
    /// Key kind of the last nudge, used together with `last_nudge_at_ms`
    /// to decide whether the next nudge merges into the existing undo
    /// entry or pushes a fresh one.
    pub last_nudge_key: Option<KeyKind>,
}

// Manual `Default` so `cache_dirty` defaults to `true`. The Task 4 test
// `body_state_default_is_idle_with_dirty_cache` asserts this; deriving
// `Default` would produce `cache_dirty: false` (Rust bool default) and
// the test would fail.
impl Default for BodyState {
    fn default() -> Self {
        Self {
            dragging: None,
            hovered_point: None,
            focused_point: None,
            pre_drag_curve: None,
            cached_path: Vec::new(),
            cached_anchors: Vec::new(),
            cache_dirty: true,
            last_nudge_at_ms: None,
            last_nudge_key: None,
        }
    }
}

/// Defensive clamp run by the body's main `use_effect` when the projected
/// curve from the live config has fewer anchors than `focused_point` indexed.
/// Originally Task 15 (external-edit reconciliation effect); the standalone
/// effect is gone (`c9e7853` deleted `EditorState.external_edit_reset`),
/// but the clamp survives as a safety net inside the cache rebuild path.
#[must_use]
pub(crate) fn clamp_focus_after_external_edit(
    state: BodyState,
    new_anchor_count: usize,
) -> BodyState {
    let mut s = state;
    s.pre_drag_curve = None;
    s.focused_point = match s.focused_point {
        Some(_) if new_anchor_count == 0 => None,
        Some(i) => Some(i.min(new_anchor_count - 1)),
        None => None,
    };
    s
}

/// In-flight drag operation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DragInProgress {
    pub point_index: usize,
    pub bounds: (f64, f64),
}

/// Flatten a curve to its draggable points in `mutation.rs` index space.
///
/// `PiecewiseLinear` / `CubicSpline`: returns `(x, y)` directly.
/// `CubicBezier`: returns `[start, c1, c2, end]` per segment, interleaved.
/// Engine-native `(input, output)` ordering throughout.
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
    fn body_state_default_is_idle_with_dirty_cache() {
        let s = BodyState::default();
        assert!(s.dragging.is_none());
        assert!(s.hovered_point.is_none());
        assert!(s.focused_point.is_none());
        assert!(s.pre_drag_curve.is_none());
        assert!(s.cache_dirty);
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
    fn clamp_focus_after_external_edit_clamps_down() {
        let s = BodyState {
            focused_point: Some(4),
            ..BodyState::default()
        };
        let next = clamp_focus_after_external_edit(s, 3);
        assert_eq!(next.focused_point, Some(2));
        assert!(next.pre_drag_curve.is_none());
    }

    #[test]
    fn clamp_focus_after_external_edit_clears_when_empty() {
        let s = BodyState {
            focused_point: Some(0),
            ..BodyState::default()
        };
        let next = clamp_focus_after_external_edit(s, 0);
        assert_eq!(next.focused_point, None);
    }

    #[test]
    fn clamp_focus_after_external_edit_noop_in_range() {
        let s = BodyState {
            focused_point: Some(1),
            ..BodyState::default()
        };
        let next = clamp_focus_after_external_edit(s, 5);
        assert_eq!(next.focused_point, Some(1));
    }
}
