// Rust guideline compliant 2026-03-04

//! Plot interaction handling for the curve editor.
//!
//! Processes hover detection, drag events, double-click point insertion,
//! and right-click point removal from `egui_plot` responses.

use egui::Pos2;
use egui_plot::{PlotPoint, PlotTransform};

use super::mutation;
use super::{CurveEditorState, HIT_RADIUS_PX};

use inputforge_core::processing::curves::ResponseCurve;

/// Process hover detection and drag events from a completed `PlotResponse`.
///
/// Updates `state.hovered_point`, `state.dragging_point`, and `state.cache_dirty`.
/// Returns `true` when a drag moved a control point.
pub(super) fn handle_plot_interaction(
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

    // Drag start, find the nearest control point within the hit radius.
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

    // Drag in progress, update the dragged point's position.
    if response.dragged() {
        if let Some(drag_idx) = state.dragging_point {
            if let Some(screen_pos) = response.hover_pos() {
                let visual_pos = plot_response.transform.value_from_position(screen_pos);
                let storage_pos = PlotPoint::new(visual_pos.y, visual_pos.x);
                let adj = mutation::adjacent_x_bounds(curve, drag_idx);
                mutation::update_point_in_curve(curve, drag_idx, storage_pos, adj);
                state.cache_dirty = true;
                changed = true;
            }
        }
    }

    // Drag ended, validate curve; revert to pre-drag snapshot on failure.
    if response.drag_stopped() && state.dragging_point.is_some() {
        if let Some(valid) = mutation::reconstruct_curve(curve) {
            *curve = valid;
        } else if let Some(snapshot) = state.pre_drag_curve.take() {
            *curve = snapshot;
        } else {
            *curve = mutation::default_identity_curve(curve);
        }
        state.cache_dirty = true;
        state.dragging_point = None;
    }

    // Double-click, add a new control point at the clicked position.
    if response.double_clicked() {
        if let Some(screen_pos) = response.hover_pos() {
            let visual_pos = plot_response.transform.value_from_position(screen_pos);
            let storage_pos = PlotPoint::new(visual_pos.y, visual_pos.x);
            if mutation::add_control_point(curve, storage_pos) {
                state.cache_dirty = true;
                changed = true;
            }
        }
    }

    // Right-click, remove the hovered control point.
    if response.secondary_clicked() {
        if let Some(hovered_idx) = state.hovered_point {
            if mutation::remove_control_point(curve, hovered_idx) {
                state.hovered_point = None;
                state.cache_dirty = true;
                changed = true;
            }
        }
    }

    changed
}

/// Find the control point nearest to `screen_pos` in screen space.
///
/// Returns `Some((index, distance_px))` when at least one point exists, or
/// `None` when `points` is empty. The caller is responsible for checking
/// whether the distance is within the desired hit radius.
pub(super) fn find_nearest_point(
    screen_pos: Pos2,
    points: &[[f64; 2]],
    transform: &PlotTransform,
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
