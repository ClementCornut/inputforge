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

/// Plot width and height in logical pixels (square canvas).
const PLOT_SIZE: f32 = 300.0;

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
    /// `true` when the cached polyline is stale and must be rebuilt.
    cache_dirty: bool,
}

impl Default for CurveEditorState {
    fn default() -> Self {
        Self {
            dragging_point: None,
            hovered_point: None,
            cached_line: Vec::new(),
            // Force a cache rebuild on first render.
            cache_dirty: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Render the interactive response curve editor.
///
/// Draws a 300 × 300 `egui_plot` canvas with draggable control points that
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
        state.cache_dirty = false;
    }

    let colors = theme::colors(ui.ctx());

    // Snapshot data needed inside the plot closure.
    // The cache is cloned because `PlotPoints::new` takes ownership of the vec.
    let control_points = extract_control_points(curve);
    let cached_line = state.cached_line.clone();
    let hovered_point = state.hovered_point;
    let dragging_point = state.dragging_point;

    let plot_id = ui.id().with("curve_editor");
    let snap = PlotSnapshot {
        control_points: &control_points,
        cached_line,
        hovered_point,
        dragging_point,
    };
    let plot_response = render_plot(ui, plot_id, curve, snap, live_input, colors);

    let changed_drag = handle_plot_interaction(curve, state, &plot_response, &control_points);

    // Rebuild now if the interaction dirtied the cache.
    if state.cache_dirty {
        rebuild_cache(curve, &mut state.cached_line);
        state.cache_dirty = false;
    }

    ui.add_space(4.0);
    let changed_controls = render_controls(ui, curve, state, colors);

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
        .data_aspect(1.0)
        .width(PLOT_SIZE)
        .height(PLOT_SIZE)
        .show_axes([true, true])
        .show_grid(true)
        .show(ui, |plot_ui| {
            // Layer 1 — identity reference line (dashed, recedes visually).
            plot_ui.line(
                Line::new("identity", PlotPoints::new(vec![[-1.0, -1.0], [1.0, 1.0]]))
                    .style(LineStyle::dashed_loose())
                    .color(colors.surface1)
                    .width(1.0),
            );

            // Layer 2 — Bezier control handles (start→c1 and c2→end segments).
            if let ResponseCurve::CubicBezier { segments, .. } = curve {
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
                        .color(colors.surface1)
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
                        .color(colors.surface1)
                        .width(1.0),
                    );
                }
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

    // Drag ended — validate curve; revert on failure.
    if response.drag_stopped() && state.dragging_point.is_some() {
        if let Some(valid) = reconstruct_curve(curve) {
            *curve = valid;
        } else {
            *curve = default_identity_curve(curve);
        }
        state.cache_dirty = true;
        state.dragging_point = None;
    }

    changed
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

/// Render control point markers for all curve types.
///
/// Uses distinct colors and sizes for default, hovered, and dragging states.
/// Bezier handles are drawn as smaller diamond shapes.
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
            // All points are anchor nodes — render as circles.
            for (idx, &pt) in control_points.iter().enumerate() {
                let (color, radius) = if Some(idx) == dragging {
                    (colors.live, 7.0)
                } else if Some(idx) == hovered {
                    (colors.primary, 7.0)
                } else {
                    (colors.text, 5.0)
                };
                plot_ui.points(
                    Points::new(format!("pt_{idx}"), PlotPoints::new(vec![pt]))
                        .shape(MarkerShape::Circle)
                        .radius(radius)
                        .color(color),
                );
            }
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            // Interleaved layout: for each segment, points are laid out as:
            // [start, control1, control2, end, ...]
            // extract_control_points produces exactly this order.
            for (idx, &pt) in control_points.iter().enumerate() {
                // Determine whether this is an anchor endpoint or a handle.
                // Segment index layout: for n segments, 4*n points total.
                // Point at position 4*k is start, 4*k+1 is c1, 4*k+2 is c2,
                // 4*k+3 is end. We expose anchors (0,3) as circles and
                // handles (1,2) as smaller diamonds.
                let local = idx % 4;
                let is_handle = local == 1 || local == 2;

                let (color, radius, shape) = if Some(idx) == dragging {
                    (
                        colors.live,
                        if is_handle { 5.0 } else { 7.0 },
                        if is_handle {
                            MarkerShape::Diamond
                        } else {
                            MarkerShape::Circle
                        },
                    )
                } else if Some(idx) == hovered {
                    (
                        colors.primary,
                        if is_handle { 5.0 } else { 7.0 },
                        if is_handle {
                            MarkerShape::Diamond
                        } else {
                            MarkerShape::Circle
                        },
                    )
                } else if is_handle {
                    (colors.text_dim, 4.0, MarkerShape::Diamond)
                } else {
                    (colors.text, 5.0, MarkerShape::Circle)
                };

                plot_ui.points(
                    Points::new(format!("bz_{idx}"), PlotPoints::new(vec![pt]))
                        .shape(shape)
                        .radius(radius)
                        .color(color),
                );
            }
            // Suppress unused variable warning for segments when no iteration needed.
            let _ = segments;
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
    colors: &theme::ThemeColors,
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

    egui::Grid::new(ui.id().with("curve_controls"))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            // Curve type selector.
            ui.label(egui::RichText::new("Type").color(colors.text_dim));
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
                        if ui
                            .selectable_value(&mut selected_type, variant, variant.label())
                            .changed()
                        {
                            // Type switching handled after closure.
                        }
                    }
                });
            if selected_type != current_type {
                if let Some(converted) = convert_curve_type(curve, selected_type) {
                    *curve = converted;
                    state.cache_dirty = true;
                    changed = true;
                }
            }
            ui.end_row();

            // Symmetry toggle.
            ui.label(egui::RichText::new("Symmetric").color(colors.text_dim));
            let mut symmetric = current_symmetric;
            if ui.checkbox(&mut symmetric, "").changed() {
                let new_curve = apply_symmetry(curve, symmetric);
                if let Some(valid) = new_curve {
                    *curve = valid;
                    state.cache_dirty = true;
                    changed = true;
                }
            }
            ui.end_row();
        });

    changed
}

// ---------------------------------------------------------------------------
// Cache helpers
// ---------------------------------------------------------------------------

/// Rebuild the 200-sample polyline cache from the current curve.
///
/// Samples are spaced evenly from -1.0 to 1.0, or from 0.0 to 1.0 when the
/// curve is symmetric (only the positive half needs distinct sampling).
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
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
}

// ---------------------------------------------------------------------------
// Drag application
// ---------------------------------------------------------------------------

/// Compute the allowed x range for moving control point at `index`.
///
/// Returns `(lower_bound, upper_bound)` exclusive so that the dragged point
/// keeps strictly between its neighbors. For Bezier handle points (non-anchor
/// positions in the 4-per-segment layout), the full `-1.0..=1.0` range is
/// returned because handles do not need to maintain x ordering.
fn adjacent_x_bounds(curve: &ResponseCurve, index: usize) -> (f64, f64) {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => {
            let lower = if index > 0 {
                points[index - 1].0 + MIN_X_GAP
            } else {
                -1.0
            };
            let upper = if index + 1 < points.len() {
                points[index + 1].0 - MIN_X_GAP
            } else {
                1.0
            };
            (lower, upper)
        }
        ResponseCurve::CubicBezier { .. } => {
            // Bezier handles have no x-ordering constraint; only endpoints
            // are loosely bounded to the visible domain.
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
fn update_point_in_curve(
    curve: &mut ResponseCurve,
    index: usize,
    new_pos: PlotPoint,
    bounds: (f64, f64),
) {
    let new_x = new_pos.x.clamp(bounds.0, bounds.1);
    let new_y = new_pos.y.clamp(-1.0, 1.0);

    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => {
            if let Some(pt) = points.get_mut(index) {
                pt.0 = new_x;
                pt.1 = new_y;
            }
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let seg_idx = index / 4;
            let local = index % 4;
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
fn default_identity_curve(curve: &ResponseCurve) -> ResponseCurve {
    match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. } => {
            let pts = if *symmetric {
                vec![(0.0, 0.0), (1.0, 1.0)]
            } else {
                vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]
            };
            ResponseCurve::piecewise_linear(pts, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                    .expect("hardcoded identity is valid")
            })
        }
        ResponseCurve::CubicSpline { symmetric, .. } => {
            let pts = if *symmetric {
                vec![(0.0, 0.0), (1.0, 1.0)]
            } else {
                vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]
            };
            ResponseCurve::cubic_spline(pts, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                    .expect("hardcoded identity is valid")
            })
        }
        ResponseCurve::CubicBezier { symmetric, .. } => {
            let seg = BezierSegment {
                start: (-1.0, -1.0),
                control1: (-1.0 / 3.0, -1.0 / 3.0),
                control2: (1.0 / 3.0, 1.0 / 3.0),
                end: (1.0, 1.0),
            };
            ResponseCurve::cubic_bezier(vec![seg], *symmetric).unwrap_or_else(|_| {
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

/// Convert a curve to a different [`CurveType`], preserving points where
/// possible.
///
/// Returns `None` when conversion produces an invalid curve (validation
/// error), leaving `curve` unchanged. The caller should not apply the result
/// in that case.
fn convert_curve_type(curve: &ResponseCurve, target: CurveType) -> Option<ResponseCurve> {
    let (points, symmetric) = match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric }
        | ResponseCurve::CubicSpline { points, symmetric } => (points.clone(), *symmetric),
        ResponseCurve::CubicBezier {
            segments,
            symmetric,
        } => {
            // Extract only the endpoint pairs (start and end of each segment).
            let mut pts: Vec<(f64, f64)> = Vec::with_capacity(segments.len() + 1);
            for (i, seg) in segments.iter().enumerate() {
                if i == 0 {
                    pts.push(seg.start);
                }
                pts.push(seg.end);
            }
            // Deduplicate: consecutive endpoints should be the same.
            pts.dedup_by(|a, b| (a.0 - b.0).abs() < MIN_X_GAP);
            (pts, *symmetric)
        }
    };

    match target {
        CurveType::PiecewiseLinear => ResponseCurve::piecewise_linear(points, symmetric).ok(),
        CurveType::CubicSpline => ResponseCurve::cubic_spline(points, symmetric).ok(),
        CurveType::CubicBezier => {
            // Build a single segment spanning the full range, using evenly
            // spaced control handles to approximate a smooth identity.
            let first = points.first().copied().unwrap_or((-1.0, -1.0));
            let last = points.last().copied().unwrap_or((1.0, 1.0));
            let dx = (last.0 - first.0) / 3.0;
            let dy = (last.1 - first.1) / 3.0;
            let seg = BezierSegment {
                start: first,
                control1: (first.0 + dx, first.1 + dy),
                control2: (last.0 - dx, last.1 - dy),
                end: last,
            };
            ResponseCurve::cubic_bezier(vec![seg], symmetric).ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Symmetry toggle
// ---------------------------------------------------------------------------

/// Apply a symmetry change to the curve, returning a validated result.
///
/// When enabling symmetry, retains only points with `x >= 0` and validates.
/// When disabling, keeps existing points unchanged (they are already valid).
/// Returns `None` when the post-change state fails validation.
fn apply_symmetry(curve: &ResponseCurve, symmetric: bool) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. } => {
            let pts = if symmetric {
                points.iter().filter(|(x, _)| *x >= 0.0).copied().collect()
            } else {
                points.clone()
            };
            ResponseCurve::piecewise_linear(pts, symmetric).ok()
        }
        ResponseCurve::CubicSpline { points, .. } => {
            let pts = if symmetric {
                points.iter().filter(|(x, _)| *x >= 0.0).copied().collect()
            } else {
                points.clone()
            };
            ResponseCurve::cubic_spline(pts, symmetric).ok()
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            // For Bezier, keep segments that are fully in the non-negative domain.
            let segs = if symmetric {
                segments
                    .iter()
                    .filter(|s| s.start.0 >= 0.0 && s.end.0 >= 0.0)
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                segments.clone()
            };
            ResponseCurve::cubic_bezier(segs, symmetric).ok()
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
    fn adjacent_x_bounds_clamps_endpoints() {
        let curve = identity_piecewise();
        // First point: no lower neighbor.
        let (lo, _) = adjacent_x_bounds(&curve, 0);
        assert!((lo - (-1.0)).abs() < f64::EPSILON);
        // Last point: no upper neighbor.
        let (_, hi) = adjacent_x_bounds(&curve, 2);
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
    fn apply_symmetry_enables_filters_negative_x() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        // Enable symmetry: must drop the x=-1.0 point and return x>=0 only.
        let result = apply_symmetry(&curve, true);
        assert!(
            result.is_some(),
            "enabling symmetry must succeed for identity"
        );
        if let Some(ResponseCurve::PiecewiseLinear { points, symmetric }) = result {
            assert!(symmetric);
            assert!(points.iter().all(|(x, _)| *x >= 0.0));
        }
    }

    #[test]
    fn apply_symmetry_disables_keeps_points() {
        let curve = ResponseCurve::piecewise_linear(vec![(0.0, 0.0), (1.0, 1.0)], true).unwrap();
        let result = apply_symmetry(&curve, false);
        assert!(result.is_some());
        if let Some(ResponseCurve::PiecewiseLinear { symmetric, .. }) = result {
            assert!(!symmetric);
        }
    }
}
