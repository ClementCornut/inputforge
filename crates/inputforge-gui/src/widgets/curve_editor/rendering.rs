// Rust guideline compliant 2026-03-04

//! Rendering functions for the curve editor plot and UI controls.
//!
//! Contains all drawing logic: the `egui_plot` canvas layers, Bezier handle
//! lines, control point markers, and the type/symmetry control strip.

use egui_plot::{HLine, Line, LineStyle, MarkerShape, Plot, PlotPoints, Points};

use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

use crate::theme;

use super::mutation;
use super::symmetry;
use super::{CurveEditorState, CurveType, PlotSnapshot};

/// Draw the `egui_plot` canvas with all rendering layers.
///
/// Returns the `PlotResponse` so the caller can access the interaction state
/// (`response` + `transform`) after the closure has returned.
pub(super) fn render_plot(
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

            // Layer 5 — live input indicator (input on Y axis).
            if let Some(input) = live_input {
                let output = curve.evaluate(input);
                plot_ui.hline(
                    HLine::new("live_input", input)
                        .color(colors.warning)
                        .width(1.5),
                );
                plot_ui.points(
                    Points::new("live_dot", PlotPoints::new(vec![[output, input]]))
                        .shape(MarkerShape::Circle)
                        .radius(6.0)
                        .color(colors.live),
                );
            }
        })
}

/// Render Bezier control handle lines (dashed) for each segment.
pub(super) fn render_bezier_handles(
    plot_ui: &mut egui_plot::PlotUi,
    segments: &[BezierSegment],
    colors: &theme::ThemeColors,
) {
    for (seg_i, seg) in segments.iter().enumerate() {
        plot_ui.line(
            Line::new(
                format!("handle_a_{seg_i}"),
                PlotPoints::new(vec![
                    [seg.start.1, seg.start.0],
                    [seg.control1.1, seg.control1.0],
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
                    [seg.control2.1, seg.control2.0],
                    [seg.end.1, seg.end.0],
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
pub(super) fn render_control_point_markers(
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
pub(super) fn render_controls(
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
            if let Some(converted) = mutation::convert_curve_type(curve, selected_type) {
                *curve = converted;
                state.cache_dirty = true;
                changed = true;
            }
        }

        ui.add_space(16.0);

        // Symmetry toggle.
        let mut symmetric = current_symmetric;
        if ui.checkbox(&mut symmetric, "Symmetric").changed() {
            let new_curve = symmetry::apply_symmetry(curve, symmetric);
            if let Some(valid) = new_curve {
                *curve = valid;
                state.cache_dirty = true;
                changed = true;
            }
        }
    });

    changed
}
