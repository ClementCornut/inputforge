// Rust guideline compliant 2026-03-04

//! Interactive response curve editor widget.
//!
//! Renders an `egui_plot` canvas with draggable control points for editing
//! [`ResponseCurve`] instances. Supports piecewise linear, cubic spline, and
//! cubic bezier curve types with optional symmetry mode. A live input marker
//! tracks real-time joystick position.

mod interaction;
mod mutation;
mod rendering;
mod symmetry;

use inputforge_core::processing::curves::{ResponseCurve, bezier_x, bezier_y};

use crate::theme;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of sample points used to draw the curve polyline.
const CURVE_SAMPLE_COUNT: usize = 200;

/// Screen-space pixel radius within which a control point is considered
/// "hit" for hover detection and drag initiation.
const HIT_RADIUS_PX: f32 = 10.0;

/// Maximum plot dimension in logical pixels. The actual size adapts to the
/// available panel width so the editor remains usable at narrow window sizes.
const PLOT_MAX_SIZE: f32 = 450.0;
/// Minimum plot dimension to keep the curve legible.
const PLOT_MIN_SIZE: f32 = 250.0;

/// Minimum x separation between adjacent control points when dragging.
const MIN_X_GAP: f64 = 0.001;

// ---------------------------------------------------------------------------
// CurveType — local enum for the ComboBox
// ---------------------------------------------------------------------------

/// Curve type discriminant used by the type-selector `ComboBox`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CurveType {
    PiecewiseLinear,
    CubicSpline,
    CubicBezier,
}

impl CurveType {
    /// Human-readable label shown in the `ComboBox`.
    const fn label(self) -> &'static str {
        match self {
            Self::PiecewiseLinear => "Piecewise Linear",
            Self::CubicSpline => "Cubic Spline",
            Self::CubicBezier => "Cubic Bezier",
        }
    }
}

// ---------------------------------------------------------------------------
// CurveEditorState
// ---------------------------------------------------------------------------

/// Per-widget persistent state for the response curve editor.
///
/// Must be stored externally (e.g., in `egui::Memory` via `ui.data_mut`)
/// and passed back to [`curve_editor`] on each frame.
#[derive(Debug, Clone)]
pub(crate) struct CurveEditorState {
    /// Index of the control point currently being dragged, `None` when idle.
    dragging_point: Option<usize>,
    /// Index of the control point nearest to the pointer (within 10 px),
    /// used for hover-highlight rendering.
    hovered_point: Option<usize>,
    /// Cached 200-sample polyline for the current curve shape.
    cached_line: Vec<[f64; 2]>,
    /// Cached flat list of draggable control point positions, rebuilt
    /// alongside `cached_line` to avoid per-frame allocation.
    cached_control_points: Vec<[f64; 2]>,
    /// `true` when the cached polyline is stale and must be rebuilt.
    cache_dirty: bool,
    /// Snapshot of the curve when a drag started, used for reverting on
    /// validation failure instead of reverting to an arbitrary previous state.
    pre_drag_curve: Option<ResponseCurve>,
}

impl Default for CurveEditorState {
    fn default() -> Self {
        Self {
            dragging_point: None,
            hovered_point: None,
            cached_line: Vec::new(),
            cached_control_points: Vec::new(),
            // Force a cache rebuild on first render.
            cache_dirty: true,
            pre_drag_curve: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Render the interactive response curve editor.
///
/// Draws a 450 × 450 `egui_plot` canvas with draggable control points that
/// modify `curve` in-place. Displays a live input indicator dot when
/// `live_input` is `Some`.
///
/// Returns `true` when the curve was modified by the user.
pub(crate) fn curve_editor(
    ui: &mut egui::Ui,
    curve: &mut ResponseCurve,
    state: &mut CurveEditorState,
    live_input: Option<f64>,
) -> bool {
    // Rebuild polyline cache when the curve changed since the last frame.
    if state.cache_dirty {
        rebuild_cache(curve, &mut state.cached_line);
        state.cached_control_points = extract_control_points(curve);
        state.cache_dirty = false;
    }

    let colors = theme::colors(ui.ctx());

    // Snapshot data needed inside the plot closure.
    // `PlotPoints::new` consumes the vec, so a clone is required here.
    // The allocation is small (~3 KB for 200 points) and unavoidable
    // because `egui_plot` does not offer a borrowing alternative.
    let cached_line = state.cached_line.clone();
    let hovered_point = state.hovered_point;
    let dragging_point = state.dragging_point;

    let plot_id = ui.id().with("curve_editor");
    let snap = PlotSnapshot {
        control_points: &state.cached_control_points,
        cached_line,
        hovered_point,
        dragging_point,
    };
    let plot_size = ui.available_width().clamp(PLOT_MIN_SIZE, PLOT_MAX_SIZE);
    let plot_response =
        rendering::render_plot(ui, plot_id, curve, snap, live_input, colors, plot_size);

    let changed_drag = interaction::handle_plot_interaction(
        curve,
        state,
        &plot_response,
        &state.cached_control_points.clone(),
    );
    plot_response
        .response
        .clone()
        .on_hover_text("Drag points \u{00b7} Double-click to add \u{00b7} Right-click to remove");

    // Rebuild now if the interaction dirtied the cache.
    if state.cache_dirty {
        rebuild_cache(curve, &mut state.cached_line);
        state.cached_control_points = extract_control_points(curve);
        state.cache_dirty = false;
    }

    ui.add_space(8.0);
    let changed_controls = rendering::render_controls(ui, curve, state);

    changed_drag || changed_controls
}

/// Frame-level snapshot passed into [`rendering::render_plot`] to avoid borrowing
/// `curve` and `state` simultaneously inside the closure.
struct PlotSnapshot<'a> {
    control_points: &'a [[f64; 2]],
    cached_line: Vec<[f64; 2]>,
    hovered_point: Option<usize>,
    dragging_point: Option<usize>,
}

// ---------------------------------------------------------------------------
// Cache helpers
// ---------------------------------------------------------------------------

/// Rebuild the polyline cache from the current curve.
///
/// For bezier curves, samples parametrically (by t) to correctly render
/// non-monotonic regions. For other curve types, samples evenly by input.
fn rebuild_cache(curve: &ResponseCurve, cached_line: &mut Vec<[f64; 2]>) {
    cached_line.clear();

    if let ResponseCurve::CubicBezier { segments, .. } = curve {
        let samples_per_seg = CURVE_SAMPLE_COUNT / segments.len().max(1);
        cached_line.reserve(samples_per_seg * segments.len());
        for seg in segments {
            let last = (samples_per_seg - 1).max(1);
            for i in 0..samples_per_seg {
                let t = i as f64 / last as f64;
                let input = bezier_x(seg, t);
                let output = bezier_y(seg, t);
                cached_line.push([output, input]);
            }
        }
        return;
    }

    cached_line.reserve(CURVE_SAMPLE_COUNT);
    let start = -1.0_f64;
    let end = 1.0_f64;
    let step = (end - start) / (CURVE_SAMPLE_COUNT - 1) as f64;
    for i in 0..CURVE_SAMPLE_COUNT {
        let x = start + i as f64 * step;
        let y = curve.evaluate(x);
        cached_line.push([y, x]);
    }
}

// ---------------------------------------------------------------------------
// Control point extraction
// ---------------------------------------------------------------------------

/// Extract the draggable control points from a curve.
///
/// For `PiecewiseLinear` / `CubicSpline`: returns the `(x, y)` point pairs.
///
/// For `CubicBezier`: returns points in segment order as
/// `[start, control1, control2, end]` per segment (interleaved, 4 per
/// segment). Adjacent endpoints from consecutive segments are returned
/// separately — the caller must not assume they are the same.
fn extract_control_points(curve: &ResponseCurve) -> Vec<[f64; 2]> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => {
            points.iter().map(|&(x, y)| [y, x]).collect()
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let mut pts = Vec::with_capacity(segments.len() * 4);
            for seg in segments {
                pts.push([seg.start.1, seg.start.0]);
                pts.push([seg.control1.1, seg.control1.0]);
                pts.push([seg.control2.1, seg.control2.0]);
                pts.push([seg.end.1, seg.end.0]);
            }
            pts
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Convenience constructors for test curves.
    fn identity_piecewise() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
    }

    fn identity_bezier() -> ResponseCurve {
        let seg = BezierSegment {
            start: (-1.0, -1.0),
            control1: (-1.0 / 3.0, -1.0 / 3.0),
            control2: (1.0 / 3.0, 1.0 / 3.0),
            end: (1.0, 1.0),
        };
        ResponseCurve::cubic_bezier(vec![seg], false).unwrap()
    }

    use inputforge_core::processing::curves::BezierSegment;

    // -----------------------------------------------------------------------

    #[test]
    fn default_state_has_dirty_cache() {
        let state = CurveEditorState::default();
        assert!(state.cache_dirty, "new state must have cache_dirty = true");
    }

    #[test]
    fn dragging_point_starts_none() {
        let state = CurveEditorState::default();
        assert!(
            state.dragging_point.is_none(),
            "dragging_point must start as None"
        );
    }

    #[test]
    fn rebuild_cache_produces_200_samples() {
        let curve = identity_piecewise();
        let mut cache = Vec::new();
        rebuild_cache(&curve, &mut cache);
        assert_eq!(
            cache.len(),
            CURVE_SAMPLE_COUNT,
            "cache must contain exactly {CURVE_SAMPLE_COUNT} samples"
        );
    }

    #[test]
    fn cache_endpoints_match_curve() {
        let curve = identity_piecewise();
        let mut cache = Vec::new();
        rebuild_cache(&curve, &mut cache);

        let tolerance = 1e-9;

        // Visual format: [output, input] where output = curve.evaluate(input).
        let first = cache[0];
        assert!(
            (first[0] - curve.evaluate(-1.0)).abs() < tolerance,
            "first sample visual-x (output) must match curve.evaluate(-1.0)"
        );
        assert!(
            (first[1] - (-1.0)).abs() < tolerance,
            "first sample visual-y (input) must be -1.0, got {}",
            first[1]
        );

        let last = cache[CURVE_SAMPLE_COUNT - 1];
        assert!(
            (last[0] - curve.evaluate(1.0)).abs() < tolerance,
            "last sample visual-x (output) must match curve.evaluate(1.0)"
        );
        assert!(
            (last[1] - 1.0).abs() < tolerance,
            "last sample visual-y (input) must be 1.0, got {}",
            last[1]
        );
    }

    #[test]
    fn extract_control_points_piecewise() {
        let curve = identity_piecewise();
        let pts = extract_control_points(&curve);
        assert_eq!(pts.len(), 3, "identity piecewise has 3 control points");
        // Visual format: [output, input]. For identity curve, output == input.
        assert!((pts[0][0] - (-1.0)).abs() < f64::EPSILON);
        assert!((pts[1][0] - 0.0).abs() < f64::EPSILON);
        assert!((pts[2][0] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_control_points_bezier() {
        let curve = identity_bezier();
        let pts = extract_control_points(&curve);
        // One segment → 4 points (start, c1, c2, end).
        assert_eq!(pts.len(), 4, "one bezier segment exposes 4 control points");
        // Visual format: [output, input]. For identity, output == input.
        assert!((pts[0][0] - (-1.0)).abs() < f64::EPSILON);
        assert!((pts[3][0] - 1.0).abs() < f64::EPSILON);
    }
}
