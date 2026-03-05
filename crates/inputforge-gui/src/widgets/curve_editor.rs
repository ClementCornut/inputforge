// Rust guideline compliant 2026-03-04

//! Interactive response curve editor widget.
//!
//! Renders an `egui_plot` canvas with draggable control points for editing
//! [`ResponseCurve`] instances. Supports piecewise linear, cubic spline, and
//! cubic bezier curve types with optional symmetry mode. A live input marker
//! tracks real-time joystick position.

use egui::Pos2;
use egui_plot::{Line, LineStyle, MarkerShape, Plot, PlotPoint, PlotPoints, Points, VLine};

use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

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
    let plot_response = render_plot(ui, plot_id, curve, snap, live_input, colors, plot_size);

    let changed_drag = handle_plot_interaction(
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
    let changed_controls = render_controls(ui, curve, state);

    changed_drag || changed_controls
}

/// Frame-level snapshot passed into [`render_plot`] to avoid borrowing
/// `curve` and `state` simultaneously inside the closure.
struct PlotSnapshot<'a> {
    control_points: &'a [[f64; 2]],
    cached_line: Vec<[f64; 2]>,
    hovered_point: Option<usize>,
    dragging_point: Option<usize>,
}

/// Draw the `egui_plot` canvas with all rendering layers.
///
/// Returns the `PlotResponse` so the caller can access the interaction state
/// (`response` + `transform`) after the closure has returned.
fn render_plot(
    ui: &mut egui::Ui,
    plot_id: egui::Id,
    curve: &ResponseCurve,
    snap: PlotSnapshot<'_>,
    live_input: Option<f64>,
    colors: &theme::ThemeColors,
    plot_size: f32,
) -> egui_plot::PlotResponse<()> {
    let PlotSnapshot {
        control_points,
        cached_line,
        hovered_point,
        dragging_point,
    } = snap;
    Plot::new(plot_id)
        .include_x(-1.1)
        .include_x(1.1)
        .include_y(-1.1)
        .include_y(1.1)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .allow_double_click_reset(false)
        .data_aspect(1.0)
        .width(plot_size)
        .height(plot_size)
        .show_axes([false, false])
        .show_grid(true)
        .show(ui, |plot_ui| {
            // Lock the view to [-1.1, 1.1] — prevent auto-scaling.
            plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max(
                [-1.1, -1.1],
                [1.1, 1.1],
            ));
            plot_ui.set_auto_bounds(egui::Vec2b::new(false, false));

            // Layer 1 — identity reference line (dashed, recedes visually).
            plot_ui.line(
                Line::new("identity", PlotPoints::new(vec![[-1.0, -1.0], [1.0, 1.0]]))
                    .style(LineStyle::dashed_loose())
                    .color(colors.surface1)
                    .width(1.0),
            );

            // Layer 2 — Bezier control handles (start→c1 and c2→end segments).
            if let ResponseCurve::CubicBezier { segments, .. } = curve {
                render_bezier_handles(plot_ui, segments, colors);
            }

            // Layer 3 — curve polyline (dominant element).
            plot_ui.line(
                Line::new("curve", PlotPoints::new(cached_line))
                    .color(colors.primary)
                    .width(2.5),
            );

            // Layer 4 — control point markers.
            render_control_point_markers(
                plot_ui,
                curve,
                control_points,
                hovered_point,
                dragging_point,
                colors,
            );

            // Layer 5 — live input indicator.
            if let Some(input) = live_input {
                let output = curve.evaluate(input);
                plot_ui.vline(
                    VLine::new("live_input", input)
                        .color(colors.warning)
                        .width(1.5),
                );
                plot_ui.points(
                    Points::new("live_dot", PlotPoints::new(vec![[input, output]]))
                        .shape(MarkerShape::Circle)
                        .radius(6.0)
                        .color(colors.live),
                );
            }
        })
}

/// Process hover detection and drag events from a completed `PlotResponse`.
///
/// Updates `state.hovered_point`, `state.dragging_point`, and `state.cache_dirty`.
/// Returns `true` when a drag moved a control point.
fn handle_plot_interaction(
    curve: &mut ResponseCurve,
    state: &mut CurveEditorState,
    plot_response: &egui_plot::PlotResponse<()>,
    control_points: &[[f64; 2]],
) -> bool {
    let mut changed = false;

    // Hover detection.
    state.hovered_point = if plot_response.response.hovered() {
        plot_response.response.hover_pos().and_then(|screen_pos| {
            find_nearest_point(screen_pos, control_points, &plot_response.transform)
                .filter(|(_, dist)| *dist <= HIT_RADIUS_PX)
                .map(|(idx, _)| idx)
        })
    } else {
        None
    };

    if state.dragging_point.is_some() {
        plot_response
            .response
            .ctx
            .output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
    } else if state.hovered_point.is_some() {
        plot_response
            .response
            .ctx
            .output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
    }

    let response = &plot_response.response;

    // Drag start — find the nearest control point within the hit radius.
    if response.drag_started() {
        let clicked_at = response
            .hover_pos()
            .or_else(|| response.interact_pointer_pos());
        state.dragging_point = clicked_at.and_then(|screen_pos| {
            find_nearest_point(screen_pos, control_points, &plot_response.transform)
                .filter(|(_, dist)| *dist <= HIT_RADIUS_PX)
                .map(|(idx, _)| idx)
        });
        // Snapshot the curve for reverting on validation failure.
        if state.dragging_point.is_some() {
            state.pre_drag_curve = Some(curve.clone());
        }
    }

    // Drag in progress — update the dragged point's position.
    if response.dragged() {
        if let Some(drag_idx) = state.dragging_point {
            if let Some(screen_pos) = response.hover_pos() {
                let new_plot_pos = plot_response.transform.value_from_position(screen_pos);
                let adj = adjacent_x_bounds(curve, drag_idx);
                update_point_in_curve(curve, drag_idx, new_plot_pos, adj);
                state.cache_dirty = true;
                changed = true;
            }
        }
    }

    // Drag ended — validate curve; revert to pre-drag snapshot on failure.
    if response.drag_stopped() && state.dragging_point.is_some() {
        if let Some(valid) = reconstruct_curve(curve) {
            *curve = valid;
        } else if let Some(snapshot) = state.pre_drag_curve.take() {
            *curve = snapshot;
        } else {
            *curve = default_identity_curve(curve);
        }
        state.cache_dirty = true;
        state.dragging_point = None;
    }

    // Double-click — add a new control point at the clicked position.
    if response.double_clicked() {
        if let Some(screen_pos) = response.hover_pos() {
            let plot_pos = plot_response.transform.value_from_position(screen_pos);
            if add_control_point(curve, plot_pos) {
                state.cache_dirty = true;
                changed = true;
            }
        }
    }

    // Right-click — remove the hovered control point.
    if response.secondary_clicked() {
        if let Some(hovered_idx) = state.hovered_point {
            if remove_control_point(curve, hovered_idx) {
                state.hovered_point = None;
                state.cache_dirty = true;
                changed = true;
            }
        }
    }

    changed
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

/// Render Bezier control handle lines (dashed) for each segment.
fn render_bezier_handles(
    plot_ui: &mut egui_plot::PlotUi,
    segments: &[BezierSegment],
    colors: &theme::ThemeColors,
) {
    for (seg_i, seg) in segments.iter().enumerate() {
        plot_ui.line(
            Line::new(
                format!("handle_a_{seg_i}"),
                PlotPoints::new(vec![
                    [seg.start.0, seg.start.1],
                    [seg.control1.0, seg.control1.1],
                ]),
            )
            .style(LineStyle::dashed_loose())
            .color(colors.text_dim.gamma_multiply(0.5))
            .width(1.0),
        );
        plot_ui.line(
            Line::new(
                format!("handle_b_{seg_i}"),
                PlotPoints::new(vec![
                    [seg.control2.0, seg.control2.1],
                    [seg.end.0, seg.end.1],
                ]),
            )
            .style(LineStyle::dashed_loose())
            .color(colors.text_dim.gamma_multiply(0.5))
            .width(1.0),
        );
    }
}

/// Render control point markers for all curve types.
///
/// Uses distinct colors and sizes for default, hovered, and dragging states.
/// Bezier handles are drawn as smaller diamond shapes. All points (including
/// the negative side of symmetric curves) are fully interactive.
fn render_control_point_markers(
    plot_ui: &mut egui_plot::PlotUi,
    curve: &ResponseCurve,
    control_points: &[[f64; 2]],
    hovered: Option<usize>,
    dragging: Option<usize>,
    colors: &theme::ThemeColors,
) {
    match curve {
        ResponseCurve::PiecewiseLinear { .. } | ResponseCurve::CubicSpline { .. } => {
            // Batch points by visual state to minimize allocations.
            let mut normal_pts = Vec::new();
            for (idx, &pt) in control_points.iter().enumerate() {
                if Some(idx) == dragging || Some(idx) == hovered {
                    // Active points rendered individually for distinct styling.
                    let (color, label) = if Some(idx) == dragging {
                        (colors.live, "pt_drag")
                    } else {
                        (colors.primary, "pt_hover")
                    };
                    plot_ui.points(
                        Points::new(label, PlotPoints::new(vec![pt]))
                            .shape(MarkerShape::Circle)
                            .radius(7.0)
                            .color(color),
                    );
                } else {
                    normal_pts.push(pt);
                }
            }
            if !normal_pts.is_empty() {
                plot_ui.points(
                    Points::new("pts", PlotPoints::new(normal_pts))
                        .shape(MarkerShape::Circle)
                        .radius(5.0)
                        .color(colors.text),
                );
            }
        }
        ResponseCurve::CubicBezier { .. } => {
            // Batch bezier points by visual state and kind (anchor vs handle).
            let mut normal_anchors = Vec::new();
            let mut normal_handles = Vec::new();
            for (idx, &pt) in control_points.iter().enumerate() {
                let local = idx % 4;
                let is_handle = local == 1 || local == 2;

                if Some(idx) == dragging || Some(idx) == hovered {
                    let color = if Some(idx) == dragging {
                        colors.live
                    } else {
                        colors.primary
                    };
                    let (radius, shape, label) = if is_handle {
                        (5.0, MarkerShape::Diamond, "bz_active_h")
                    } else {
                        (7.0, MarkerShape::Circle, "bz_active_a")
                    };
                    plot_ui.points(
                        Points::new(label, PlotPoints::new(vec![pt]))
                            .shape(shape)
                            .radius(radius)
                            .color(color),
                    );
                } else if is_handle {
                    normal_handles.push(pt);
                } else {
                    normal_anchors.push(pt);
                }
            }
            if !normal_anchors.is_empty() {
                plot_ui.points(
                    Points::new("bz_anchors", PlotPoints::new(normal_anchors))
                        .shape(MarkerShape::Circle)
                        .radius(5.0)
                        .color(colors.text),
                );
            }
            if !normal_handles.is_empty() {
                plot_ui.points(
                    Points::new("bz_handles", PlotPoints::new(normal_handles))
                        .shape(MarkerShape::Diamond)
                        .radius(4.0)
                        .color(colors.text_dim),
                );
            }
        }
    }
}

/// Render curve type `ComboBox` and symmetry `Checkbox` below the plot.
///
/// Returns `true` if either control changed the curve.
fn render_controls(
    ui: &mut egui::Ui,
    curve: &mut ResponseCurve,
    state: &mut CurveEditorState,
) -> bool {
    let mut changed = false;

    let current_type = match curve {
        ResponseCurve::PiecewiseLinear { .. } => CurveType::PiecewiseLinear,
        ResponseCurve::CubicSpline { .. } => CurveType::CubicSpline,
        ResponseCurve::CubicBezier { .. } => CurveType::CubicBezier,
    };

    let current_symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };

    ui.horizontal(|ui| {
        // Curve type selector.
        let mut selected_type = current_type;
        egui::ComboBox::from_id_salt(ui.id().with("curve_type"))
            .selected_text(selected_type.label())
            .width(150.0)
            .show_ui(ui, |ui| {
                for variant in [
                    CurveType::PiecewiseLinear,
                    CurveType::CubicSpline,
                    CurveType::CubicBezier,
                ] {
                    ui.selectable_value(&mut selected_type, variant, variant.label());
                }
            });
        if selected_type != current_type {
            if let Some(converted) = convert_curve_type(curve, selected_type) {
                *curve = converted;
                state.cache_dirty = true;
                changed = true;
            }
        }

        ui.add_space(16.0);

        // Symmetry toggle.
        let mut symmetric = current_symmetric;
        if ui.checkbox(&mut symmetric, "Symmetric").changed() {
            let new_curve = apply_symmetry(curve, symmetric);
            if let Some(valid) = new_curve {
                *curve = valid;
                state.cache_dirty = true;
                changed = true;
            }
        }
    });

    changed
}

// ---------------------------------------------------------------------------
// Cache helpers
// ---------------------------------------------------------------------------

/// Rebuild the 200-sample polyline cache from the current curve.
///
/// Samples are spaced evenly from -1.0 to 1.0.
fn rebuild_cache(curve: &ResponseCurve, cached_line: &mut Vec<[f64; 2]>) {
    cached_line.clear();
    cached_line.reserve(CURVE_SAMPLE_COUNT);

    let start = -1.0_f64;
    let end = 1.0_f64;
    let step = (end - start) / (CURVE_SAMPLE_COUNT - 1) as f64;

    for i in 0..CURVE_SAMPLE_COUNT {
        let x = start + i as f64 * step;
        let y = curve.evaluate(x);
        cached_line.push([x, y]);
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
            points.iter().map(|&(x, y)| [x, y]).collect()
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let mut pts = Vec::with_capacity(segments.len() * 4);
            for seg in segments {
                pts.push([seg.start.0, seg.start.1]);
                pts.push([seg.control1.0, seg.control1.1]);
                pts.push([seg.control2.0, seg.control2.1]);
                pts.push([seg.end.0, seg.end.1]);
            }
            pts
        }
    }
}

// ---------------------------------------------------------------------------
// Nearest-point search
// ---------------------------------------------------------------------------

/// Find the control point nearest to `screen_pos` in screen space.
///
/// Returns `Some((index, distance_px))` when at least one point exists, or
/// `None` when `points` is empty. The caller is responsible for checking
/// whether the distance is within the desired hit radius.
fn find_nearest_point(
    screen_pos: Pos2,
    points: &[[f64; 2]],
    transform: &egui_plot::PlotTransform,
) -> Option<(usize, f32)> {
    points
        .iter()
        .enumerate()
        .map(|(idx, &[x, y])| {
            let plot_pt = PlotPoint::new(x, y);
            let screen_pt = transform.position_from_point(&plot_pt);
            let dist = screen_pos.distance(screen_pt);
            (idx, dist)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

// ---------------------------------------------------------------------------
// Drag application
// ---------------------------------------------------------------------------

/// Compute the allowed x range for moving control point at `index`.
///
/// Returns `(lower_bound, upper_bound)` exclusive so that the dragged point
/// keeps strictly between its neighbors. Edge points (first and last) have
/// their x position locked. In symmetric mode, the center point is frozen
/// at x = 0.
fn adjacent_x_bounds(curve: &ResponseCurve, index: usize) -> (f64, f64) {
    let symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };

    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => {
            let count = points.len();

            // Edge points: x locked at their current position.
            if index == 0 {
                return (points[0].0, points[0].0);
            }
            if index == count - 1 {
                return (points[count - 1].0, points[count - 1].0);
            }

            // Center point in symmetric mode: frozen at x = 0.
            if symmetric && count % 2 == 1 && index == count / 2 {
                return (0.0, 0.0);
            }

            let lower = points[index - 1].0 + MIN_X_GAP;
            let upper = points[index + 1].0 - MIN_X_GAP;
            (lower, upper)
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let seg_idx = index / 4;
            let local = index % 4;
            let last_seg = segments.len().saturating_sub(1);

            // Lock endpoint x: first segment start at x=-1, last segment end at x=1.
            if seg_idx == 0 && local == 0 {
                return (-1.0, -1.0);
            }
            if seg_idx == last_seg && local == 3 {
                return (1.0, 1.0);
            }

            // All other bezier handles are unconstrained.
            (-1.0, 1.0)
        }
    }
}

/// Update a single control point in the curve, clamping x to `bounds`.
///
/// For `PiecewiseLinear` / `CubicSpline` the matching point tuple is updated
/// directly.  For `CubicBezier` the point at the given index within the
/// interleaved `[start, c1, c2, end]` layout is updated in the corresponding
/// segment field.
///
/// When the curve is symmetric, the mirror point at `count - 1 - index` is
/// automatically updated to `(-x, -y)`, maintaining antisymmetry.
fn update_point_in_curve(
    curve: &mut ResponseCurve,
    index: usize,
    new_pos: PlotPoint,
    bounds: (f64, f64),
) {
    let new_x = new_pos.x.clamp(bounds.0, bounds.1);
    let new_y = new_pos.y.clamp(-1.0, 1.0);

    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            // Center point is frozen at (0, 0) in symmetric mode.
            if *symmetric && points.len() % 2 == 1 && index == points.len() / 2 {
                return;
            }
            if let Some(pt) = points.get_mut(index) {
                pt.0 = new_x;
                pt.1 = new_y;
            }
            // Auto-mirror in symmetric mode.
            if *symmetric {
                let count = points.len();
                let mirror_idx = count - 1 - index;
                if mirror_idx != index {
                    if let Some(mirror_pt) = points.get_mut(mirror_idx) {
                        mirror_pt.0 = -new_x;
                        mirror_pt.1 = -new_y;
                    }
                }
            }
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => update_bezier_point(segments, *symmetric, index, new_x, new_y),
    }
}

/// Update a single bezier control point and enforce symmetric mirroring.
///
/// Handles center-freeze, endpoint sync between consecutive segments,
/// and antisymmetric mirroring when `symmetric` is enabled.
fn update_bezier_point(
    segments: &mut [BezierSegment],
    symmetric: bool,
    index: usize,
    new_x: f64,
    new_y: f64,
) {
    let seg_idx = index / 4;
    let local = index % 4;

    // Center junction point is frozen at (0, 0) in symmetric mode.
    // For N segments, the center is at segment N/2, local 0 (= start).
    if symmetric && segments.len() % 2 == 0 {
        let center_seg = segments.len() / 2;
        if seg_idx == center_seg && local == 0 {
            return;
        }
        // Also block the alias: previous segment's end (local 3).
        if seg_idx == center_seg - 1 && local == 3 {
            return;
        }
    }

    if let Some(seg) = segments.get_mut(seg_idx) {
        match local {
            0 => {
                seg.start.0 = new_x;
                seg.start.1 = new_y;
            }
            1 => {
                seg.control1.0 = new_x;
                seg.control1.1 = new_y;
            }
            2 => {
                seg.control2.0 = new_x;
                seg.control2.1 = new_y;
            }
            3 => {
                seg.end.0 = new_x;
                seg.end.1 = new_y;
            }
            _ => {}
        }
    }

    // Sync shared endpoints between consecutive segments:
    // segment N's end == segment N+1's start.
    if local == 3 {
        if let Some(next) = segments.get_mut(seg_idx + 1) {
            next.start.0 = new_x;
            next.start.1 = new_y;
        }
    } else if local == 0 && seg_idx > 0 {
        if let Some(prev) = segments.get_mut(seg_idx - 1) {
            prev.end.0 = new_x;
            prev.end.1 = new_y;
        }
    }

    // Auto-mirror in symmetric mode: mirror the corresponding
    // point in the opposite segment. For N segments, segment i
    // mirrors to segment (N - 1 - i), with local positions
    // swapped (0<->3, 1<->2).
    if symmetric {
        let seg_count = segments.len();
        let mirror_seg_idx = seg_count - 1 - seg_idx;
        let mirror_local = 3 - local;

        // Track which segment the primary endpoint sync touched,
        // so we skip overlapping mirror endpoint sync.
        let primary_synced_idx = match local {
            3 => Some(seg_idx + 1),
            0 if seg_idx > 0 => Some(seg_idx - 1),
            _ => None,
        };

        // Only mirror if it is a different point (not the center).
        if mirror_seg_idx != seg_idx || mirror_local != local {
            if let Some(mirror_seg) = segments.get_mut(mirror_seg_idx) {
                match mirror_local {
                    0 => mirror_seg.start = (-new_x, -new_y),
                    1 => mirror_seg.control1 = (-new_x, -new_y),
                    2 => mirror_seg.control2 = (-new_x, -new_y),
                    3 => mirror_seg.end = (-new_x, -new_y),
                    _ => {}
                }
            }
            // Sync shared endpoints for the mirrored segment,
            // skipping if the primary sync already wrote to this segment.
            if mirror_local == 3 {
                let target = mirror_seg_idx + 1;
                if primary_synced_idx != Some(target) {
                    if let Some(next) = segments.get_mut(target) {
                        next.start = (-new_x, -new_y);
                    }
                }
            } else if mirror_local == 0 && mirror_seg_idx > 0 {
                let target = mirror_seg_idx - 1;
                if primary_synced_idx != Some(target) {
                    if let Some(prev) = segments.get_mut(target) {
                        prev.end = (-new_x, -new_y);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Curve reconstruction after drag
// ---------------------------------------------------------------------------

/// Reconstruct a fully validated curve from the current (potentially dirty)
/// internal state.
///
/// Returns `None` when the state fails validation (e.g., points out of order
/// after a drag). The caller should revert to a safe default in that case.
fn reconstruct_curve(curve: &ResponseCurve) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric } => {
            ResponseCurve::piecewise_linear(points.clone(), *symmetric).ok()
        }
        ResponseCurve::CubicSpline { points, symmetric } => {
            ResponseCurve::cubic_spline(points.clone(), *symmetric).ok()
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => ResponseCurve::cubic_bezier(segments.clone(), *symmetric).ok(),
    }
}

/// Return a safe identity fallback with the same type and symmetry as `curve`.
///
/// Symmetric curves store all points on both sides of the origin.
fn default_identity_curve(curve: &ResponseCurve) -> ResponseCurve {
    match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. } => {
            let pts = vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)];
            ResponseCurve::piecewise_linear(pts, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                    .expect("hardcoded identity is valid")
            })
        }
        ResponseCurve::CubicSpline { symmetric, .. } => {
            let pts = vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)];
            ResponseCurve::cubic_spline(pts, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                    .expect("hardcoded identity is valid")
            })
        }
        ResponseCurve::CubicBezier { symmetric, .. } => {
            let segs = if *symmetric {
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
            };
            ResponseCurve::cubic_bezier(segs, *symmetric).unwrap_or_else(|_| {
                let fallback_seg = BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-1.0 / 3.0, -1.0 / 3.0),
                    control2: (1.0 / 3.0, 1.0 / 3.0),
                    end: (1.0, 1.0),
                };
                ResponseCurve::cubic_bezier(vec![fallback_seg], false)
                    .expect("hardcoded bezier identity is valid")
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Curve type conversion
// ---------------------------------------------------------------------------

/// Convert a curve to a different [`CurveType`] by creating fresh defaults.
///
/// Matches `JoystickGremlin` behavior: switching types resets the curve to
/// a clean identity rather than attempting to preserve arbitrary point
/// configurations. Preserves the symmetric flag and applies enforcement if
/// symmetric.
fn convert_curve_type(curve: &ResponseCurve, target: CurveType) -> Option<ResponseCurve> {
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
            let segs = if symmetric {
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
            };
            ResponseCurve::cubic_bezier(segs, symmetric).ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Add / remove control points
// ---------------------------------------------------------------------------

/// Add a control point at `pos` on the curve. For symmetric curves, also
/// adds the mirror point at `(-x, -y)`.
///
/// Returns `true` when the point was added successfully.
fn add_control_point(curve: &mut ResponseCurve, pos: PlotPoint) -> bool {
    let x = pos.x.clamp(-1.0, 1.0);
    let y = pos.y.clamp(-1.0, 1.0);

    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            let original_points = points.clone();
            points.push((x, y));
            if *symmetric {
                // Add mirror point (skip if at origin).
                if x.abs() > 0.0 {
                    points.push((-x, -y));
                }
            }
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            // Validate: check x values are strictly increasing after sort.
            if points.windows(2).all(|w| w[0].0 < w[1].0) {
                true
            } else {
                // Rollback: restore original points on validation failure.
                *points = original_points;
                false
            }
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            // Find the segment containing x.
            let Some(seg_idx) = segments.iter().position(|s| s.start.0 <= x && x <= s.end.0) else {
                return false;
            };

            // Compute t parameter (linear approximation).
            let seg = &segments[seg_idx];
            let dx = seg.end.0 - seg.start.0;
            if dx.abs() < f64::EPSILON {
                return false;
            }
            let t = ((x - seg.start.0) / dx).clamp(0.05, 0.95);

            // De Casteljau split.
            let (left, right) = split_bezier_segment(seg, t);
            segments.splice(seg_idx..=seg_idx, [left, right]);

            // Mirror in symmetric mode.
            if *symmetric {
                // segments.len() is post-splice (original + 1).
                let pre_splice_count = segments.len() - 1;
                let mut mirror_seg = pre_splice_count - 1 - seg_idx;
                // Adjust mirror index for the splice insertion at seg_idx.
                if mirror_seg >= seg_idx {
                    mirror_seg += 1;
                }
                if mirror_seg != seg_idx && mirror_seg != seg_idx + 1 {
                    // Compute mirror t from the mirror segment's geometry
                    // so the split point lands at (-x, -y) for antisymmetry.
                    let mirror_x = -x;
                    let m_seg = &segments[mirror_seg];
                    let m_dx = m_seg.end.0 - m_seg.start.0;
                    let mirror_t = if m_dx.abs() < f64::EPSILON {
                        0.5
                    } else {
                        ((mirror_x - m_seg.start.0) / m_dx).clamp(0.05, 0.95)
                    };
                    let (ml, mr) = split_bezier_segment(&segments[mirror_seg], mirror_t);
                    segments.splice(mirror_seg..=mirror_seg, [ml, mr]);
                }
            }
            true
        }
    }
}

/// Remove control point at `index`. For symmetric curves, also removes the
/// mirror point. Edge points and center point cannot be removed.
///
/// Returns `true` when the point was removed successfully.
fn remove_control_point(curve: &mut ResponseCurve, index: usize) -> bool {
    match curve {
        ResponseCurve::PiecewiseLinear {
            points, symmetric, ..
        }
        | ResponseCurve::CubicSpline {
            points, symmetric, ..
        } => {
            let count = points.len();

            // Cannot remove edge points.
            if index == 0 || index == count - 1 {
                return false;
            }
            // Cannot remove center point in symmetric mode.
            if *symmetric && count % 2 == 1 && index == count / 2 {
                return false;
            }
            // Need at least 2 points after removal.
            let removals = if *symmetric { 2 } else { 1 };
            if count <= removals + 1 {
                return false;
            }

            if *symmetric {
                let mirror_idx = count - 1 - index;
                // Center point (index == mirror_idx) is already blocked above.
                debug_assert_ne!(index, mirror_idx, "center removal should be caught earlier");
                // Remove higher index first to avoid shifting.
                let (first, second) = if index > mirror_idx {
                    (index, mirror_idx)
                } else {
                    (mirror_idx, index)
                };
                points.remove(first);
                points.remove(second);
            } else {
                points.remove(index);
            }
            true
        }
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            let seg_idx = index / 4;
            let local = index % 4;

            // Only junction points (endpoints shared between segments) can be removed.
            // Control handles (local 1, 2) cannot be removed independently.
            if local == 1 || local == 2 {
                return false;
            }

            // Determine which two segments share this junction.
            let (left_idx, right_idx) = if local == 3 {
                (seg_idx, seg_idx + 1)
            } else {
                // local == 0
                if seg_idx == 0 {
                    return false; // First start point — edge.
                }
                (seg_idx - 1, seg_idx)
            };

            let seg_count = segments.len();
            if right_idx >= seg_count {
                return false; // Last end point — edge.
            }
            // Need at least 1 segment after merge.
            if seg_count < 2 {
                return false;
            }

            // Cannot remove center junction in symmetric mode.
            if *symmetric && seg_count % 2 == 0 {
                let center_seg = seg_count / 2;
                if (local == 3 && seg_idx == center_seg - 1)
                    || (local == 0 && seg_idx == center_seg)
                {
                    return false;
                }
            }

            // Merge: keep left's start+control1, right's control2+end.
            let merged = BezierSegment {
                start: segments[left_idx].start,
                control1: segments[left_idx].control1,
                control2: segments[right_idx].control2,
                end: segments[right_idx].end,
            };
            segments.splice(left_idx..=right_idx, [merged]);

            // Mirror in symmetric mode.
            if *symmetric {
                // The primary merge replaced 2 segments with 1, so
                // pre_merge_count = current + 1.
                let pre_merge_count = segments.len() + 1;
                let mut mirror_left = pre_merge_count - 2 - left_idx;
                // Adjust: the merge at left_idx reduced indices above it by 1.
                if mirror_left > left_idx {
                    mirror_left -= 1;
                }
                let new_count = segments.len();
                if mirror_left < new_count && mirror_left != left_idx {
                    let mirror_right = mirror_left + 1;
                    if mirror_right < new_count {
                        let mirror_merged = BezierSegment {
                            start: segments[mirror_left].start,
                            control1: segments[mirror_left].control1,
                            control2: segments[mirror_right].control2,
                            end: segments[mirror_right].end,
                        };
                        segments.splice(mirror_left..=mirror_right, [mirror_merged]);
                    }
                }
            }
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Bezier helpers
// ---------------------------------------------------------------------------

/// Linearly interpolate between two 2D points.
fn lerp_point(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Split a cubic Bezier segment at parameter `t` using De Casteljau's algorithm.
///
/// Returns the two sub-segments `(left, right)` whose union equals the original.
fn split_bezier_segment(seg: &BezierSegment, t: f64) -> (BezierSegment, BezierSegment) {
    // Level 1: interpolate between adjacent original control points.
    let ab = lerp_point(seg.start, seg.control1, t);
    let bc = lerp_point(seg.control1, seg.control2, t);
    let cd = lerp_point(seg.control2, seg.end, t);
    // Level 2: interpolate between level-1 results.
    let abc = lerp_point(ab, bc, t);
    let bcd = lerp_point(bc, cd, t);
    // Level 3: the point on the curve at parameter t.
    let mid = lerp_point(abc, bcd, t);

    let left = BezierSegment {
        start: seg.start,
        control1: ab,
        control2: abc,
        end: mid,
    };
    let right = BezierSegment {
        start: mid,
        control1: bcd,
        control2: cd,
        end: seg.end,
    };
    (left, right)
}

// ---------------------------------------------------------------------------
// Symmetry toggle
// ---------------------------------------------------------------------------

/// Apply a symmetry change to the curve, returning a validated result.
///
/// When **enabling** symmetry, restructures the curve to be antisymmetric
/// through the origin by mirroring the positive-half points to the negative
/// side, matching `JoystickGremlin` `_enforce_symmetry()` behavior.
///
/// When **disabling** symmetry, simply clears the flag — all points (both
/// sides) are kept as-is.
///
/// Returns `None` when the post-change state fails validation.
fn apply_symmetry(curve: &ResponseCurve, symmetric: bool) -> Option<ResponseCurve> {
    if symmetric {
        // Enabling: enforce antisymmetry.
        enforce_symmetry(curve)
    } else {
        // Disabling: mutate the flag in-place without cloning or re-validating.
        let mut result = curve.clone();
        result.set_symmetric(false);
        Some(result)
    }
}

/// Enforce antisymmetry on a curve: `f(-x) = -f(x)`.
///
/// Takes the positive-half points (x >= 0), mirrors them to create the
/// negative side, and ensures the origin is included. Matches
/// `JoystickGremlin` `_enforce_symmetry()`.
fn enforce_symmetry(curve: &ResponseCurve) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. } => {
            let pts = enforce_symmetry_points(points);
            ResponseCurve::piecewise_linear(pts, true).ok()
        }
        ResponseCurve::CubicSpline { points, .. } => {
            let pts = enforce_symmetry_points(points);
            ResponseCurve::cubic_spline(pts, true).ok()
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let segs = enforce_symmetry_bezier(segments);
            ResponseCurve::cubic_bezier(segs, true).ok()
        }
    }
}

/// Build a full antisymmetric point set from existing points.
///
/// Keeps points with x >= 0, mirrors them to the negative side, and
/// ensures the origin (0, 0) is present. If no positive-side points
/// exist, falls back to a minimal identity.
fn enforce_symmetry_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    // Collect positive-half points (x >= 0), sorted by x.
    let mut positive: Vec<(f64, f64)> = points.iter().filter(|(x, _)| *x >= 0.0).copied().collect();
    positive.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Ensure origin is present.
    if positive.is_empty() || positive[0].0 > 0.0 {
        positive.insert(0, (0.0, 0.0));
    } else {
        // Lock origin y to 0 for antisymmetry.
        positive[0].1 = 0.0;
    }

    // Ensure at least (0,0) and (1,1).
    if positive.len() < 2 {
        positive.push((1.0, 1.0));
    }

    // Mirror positive points (excluding origin) to negative side.
    let mut result: Vec<(f64, f64)> = positive
        .iter()
        .filter(|(x, _)| *x > 0.0)
        .map(|(x, y)| (-x, -y))
        .collect();
    result.reverse();
    result.extend_from_slice(&positive);
    result
}

/// Build a full antisymmetric bezier segment set from existing segments.
///
/// Keeps segments in the positive domain and mirrors them to the negative
/// side. If no positive segments exist, creates a default symmetric pair.
fn enforce_symmetry_bezier(segments: &[BezierSegment]) -> Vec<BezierSegment> {
    // Collect segments with start.x >= 0.
    let positive: Vec<_> = segments
        .iter()
        .filter(|s| s.start.0 >= 0.0)
        .cloned()
        .collect();

    let positive = if positive.is_empty() {
        // Fallback: create a default positive segment.
        vec![BezierSegment {
            start: (0.0, 0.0),
            control1: (1.0 / 3.0, 1.0 / 3.0),
            control2: (2.0 / 3.0, 2.0 / 3.0),
            end: (1.0, 1.0),
        }]
    } else {
        positive
    };

    // Mirror positive segments to create negative side.
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

    fn identity_spline() -> ResponseCurve {
        ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
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

        let first = cache[0];
        assert!(
            (first[0] - (-1.0)).abs() < tolerance,
            "first sample x must be -1.0, got {}",
            first[0]
        );
        assert!(
            (first[1] - curve.evaluate(-1.0)).abs() < tolerance,
            "first sample y must match curve.evaluate(-1.0)"
        );

        let last = cache[CURVE_SAMPLE_COUNT - 1];
        assert!(
            (last[0] - 1.0).abs() < tolerance,
            "last sample x must be 1.0, got {}",
            last[0]
        );
        assert!(
            (last[1] - curve.evaluate(1.0)).abs() < tolerance,
            "last sample y must match curve.evaluate(1.0)"
        );
    }

    #[test]
    fn extract_control_points_piecewise() {
        let curve = identity_piecewise();
        let pts = extract_control_points(&curve);
        assert_eq!(pts.len(), 3, "identity piecewise has 3 control points");
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
        // start
        assert!((pts[0][0] - (-1.0)).abs() < f64::EPSILON);
        // end
        assert!((pts[3][0] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn convert_curve_type_preserves_identity() {
        let curve = identity_piecewise();
        let tolerance = 0.02; // Conversion may introduce slight shape differences.

        // Piecewise → Spline.
        let spline = convert_curve_type(&curve, CurveType::CubicSpline)
            .expect("conversion to spline must succeed");
        assert!((spline.evaluate(0.0)).abs() < tolerance);
        assert!((spline.evaluate(1.0) - 1.0).abs() < tolerance);

        // Piecewise → Bezier.
        let bezier = convert_curve_type(&curve, CurveType::CubicBezier)
            .expect("conversion to bezier must succeed");
        assert!((bezier.evaluate(0.0)).abs() < tolerance);

        // Piecewise → Piecewise (identity conversion).
        let same = convert_curve_type(&curve, CurveType::PiecewiseLinear)
            .expect("same-type conversion must succeed");
        assert!((same.evaluate(0.5) - 0.5).abs() < tolerance);
    }

    #[test]
    fn convert_spline_to_bezier() {
        let curve = identity_spline();
        let bezier = convert_curve_type(&curve, CurveType::CubicBezier)
            .expect("spline-to-bezier conversion must succeed");
        assert!((bezier.evaluate(-1.0) - (-1.0)).abs() < 0.01);
        assert!((bezier.evaluate(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn adjacent_x_bounds_locks_edge_points() {
        let curve = identity_piecewise();
        // First point: x locked at -1.0.
        let (lo, hi) = adjacent_x_bounds(&curve, 0);
        assert!((lo - (-1.0)).abs() < f64::EPSILON);
        assert!((hi - (-1.0)).abs() < f64::EPSILON);
        // Last point: x locked at 1.0.
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 1.0).abs() < f64::EPSILON);
        assert!((hi - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adjacent_x_bounds_middle_point() {
        let curve = identity_piecewise();
        let (lo, hi) = adjacent_x_bounds(&curve, 1);
        // Lower bound is the x of point[0] + MIN_X_GAP.
        assert!((lo - (-1.0 + MIN_X_GAP)).abs() < 1e-10);
        // Upper bound is the x of point[2] - MIN_X_GAP.
        assert!((hi - (1.0 - MIN_X_GAP)).abs() < 1e-10);
    }

    #[test]
    fn default_identity_curve_is_valid_for_each_type() {
        let pw = default_identity_curve(&identity_piecewise());
        let sp = default_identity_curve(&identity_spline());
        let bz = default_identity_curve(&identity_bezier());
        // Must evaluate without panicking and produce finite values.
        assert!(pw.evaluate(0.0).is_finite());
        assert!(sp.evaluate(0.0).is_finite());
        assert!(bz.evaluate(0.0).is_finite());
    }

    #[test]
    fn apply_symmetry_enforces_antisymmetric_points() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let result = apply_symmetry(&curve, true);
        assert!(
            result.is_some(),
            "enabling symmetry must succeed for identity"
        );
        if let Some(ResponseCurve::PiecewiseLinear { points, symmetric }) = result {
            assert!(symmetric);
            // Must have origin and mirrored points on both sides.
            assert!(points.len() >= 3);
            // Origin must be at (0, 0).
            let center = points.iter().find(|(x, _)| x.abs() < f64::EPSILON);
            assert!(center.is_some(), "origin must be present");
            assert!(
                (center.unwrap().1).abs() < f64::EPSILON,
                "origin y must be 0"
            );
        }
    }

    #[test]
    fn apply_symmetry_two_point_default_curve() {
        // The default identity curve [(-1,-1), (1,1)] must produce a valid
        // symmetric curve with origin and mirrored points on both sides.
        let curve = ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
        let result = apply_symmetry(&curve, true);
        assert!(
            result.is_some(),
            "enabling symmetry on 2-point default curve must succeed"
        );
        if let Some(ResponseCurve::PiecewiseLinear { points, symmetric }) = result {
            assert!(symmetric);
            assert!(
                points.len() >= 3,
                "symmetric curve must have at least 3 points (neg, origin, pos), got {}",
                points.len()
            );
            // First point must be negative, last must be positive.
            assert!(points[0].0 < 0.0, "first point must be negative x");
            assert!(
                points[points.len() - 1].0 > 0.0,
                "last point must be positive x"
            );
        }
    }

    #[test]
    fn apply_symmetry_disable_keeps_all_points() {
        // Symmetric curve with full-range points; disabling just clears the flag.
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (0.5, 0.2), (1.0, 1.0)],
            true,
        )
        .unwrap();
        let result = apply_symmetry(&curve, false);
        assert!(result.is_some(), "disabling symmetry must succeed");
        if let Some(ResponseCurve::PiecewiseLinear { points, symmetric }) = result {
            assert!(!symmetric);
            // All original points must be preserved.
            assert_eq!(points.len(), 4);
        }
    }

    #[test]
    fn adjacent_x_bounds_edge_points_locked() {
        let curve = identity_piecewise();
        // First point: x locked at -1.0.
        let (lo, hi) = adjacent_x_bounds(&curve, 0);
        assert!((lo - (-1.0)).abs() < f64::EPSILON, "first point lo = -1.0");
        assert!((hi - (-1.0)).abs() < f64::EPSILON, "first point hi = -1.0");
        // Last point: x locked at 1.0.
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 1.0).abs() < f64::EPSILON, "last point lo = 1.0");
        assert!((hi - 1.0).abs() < f64::EPSILON, "last point hi = 1.0");
    }

    #[test]
    fn adjacent_x_bounds_symmetric_locks_center() {
        let curve = ResponseCurve::piecewise_linear(
            vec![
                (-1.0, -1.0),
                (-0.5, -0.3),
                (0.0, 0.0),
                (0.5, 0.3),
                (1.0, 1.0),
            ],
            true,
        )
        .unwrap();
        // Center point (index 2) must be locked at x = 0.
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 0.0).abs() < f64::EPSILON, "center lo must be 0.0");
        assert!((hi - 0.0).abs() < f64::EPSILON, "center hi must be 0.0");
    }

    #[test]
    fn enforce_symmetry_points_produces_antisymmetric() {
        let points = vec![(-1.0, -0.8), (0.0, 0.1), (0.5, 0.3), (1.0, 1.0)];
        let result = enforce_symmetry_points(&points);
        // Must have mirrored positive side to negative and fixed origin y to 0.
        assert!(result.len() >= 5);
        // Check antisymmetry: for each positive point, a mirrored negative must exist.
        for &(x, y) in &result {
            if x.abs() > f64::EPSILON {
                let mirror = result.iter().find(|(mx, _)| (mx + x).abs() < f64::EPSILON);
                assert!(mirror.is_some(), "mirror of ({x}, {y}) must exist");
                let (_, my) = mirror.unwrap();
                assert!(
                    (my + y).abs() < f64::EPSILON,
                    "mirror y must be -{y}, got {my}"
                );
            }
        }
    }

    #[test]
    fn center_point_frozen_in_symmetric_mode() {
        use egui_plot::PlotPoint;

        let mut curve = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            symmetric: true,
        };
        // Try to drag center point (index 1) to (0.3, 0.5).
        let bounds = adjacent_x_bounds(&curve, 1);
        update_point_in_curve(&mut curve, 1, PlotPoint::new(0.3, 0.5), bounds);

        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert!(
                points[1].0.abs() < f64::EPSILON,
                "center x must stay at 0, got {}",
                points[1].0
            );
            assert!(
                points[1].1.abs() < f64::EPSILON,
                "center y must stay at 0, got {}",
                points[1].1
            );
        } else {
            panic!("expected PiecewiseLinear");
        }
    }
}
