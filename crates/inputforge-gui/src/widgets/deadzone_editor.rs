// Rust guideline compliant 2026-03-04

//! Visual deadzone configuration editor widget.
//!
//! Renders a custom-painted horizontal bar showing the five deadzone zones
//! with optional live input marker, plus four `DragValue` sliders for
//! editing the thresholds.

use egui::{FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::theme;

/// Width of the visual preview bar in logical pixels.
const BAR_WIDTH: f32 = 200.0;

/// Height of the visual preview bar in logical pixels.
const BAR_HEIGHT: f32 = 30.0;

/// Drag speed for threshold sliders.
const DRAG_SPEED: f64 = 0.01;

/// Render a single label + `DragValue` row inside a `Grid`.
///
/// Returns `true` when the value was changed by the user.
fn drag_row(ui: &mut egui::Ui, label: &str, value: &mut f64) -> bool {
    let colors = theme::colors(ui.ctx());
    ui.label(egui::RichText::new(label).color(colors.text_dim));
    let changed = ui
        .add(
            egui::DragValue::new(value)
                .range(-1.0..=1.0)
                .speed(DRAG_SPEED)
                .fixed_decimals(2),
        )
        .changed();
    ui.end_row();
    changed
}

/// Render the deadzone editor widget.
///
/// Displays a visual preview bar and four `DragValue` controls.
/// Returns `Some(new_config)` when the user modifies a value and the
/// new configuration is valid. Returns `None` when nothing changed.
pub(crate) fn deadzone_editor(
    ui: &mut egui::Ui,
    config: &DeadzoneConfig,
    live_input: Option<f64>,
) -> Option<DeadzoneConfig> {
    // Paint visual preview.
    paint_deadzone_bar(ui, config, live_input);

    ui.add_space(4.0);

    // Editable thresholds.
    let mut low = config.low();
    let mut center_low = config.center_low();
    let mut center_high = config.center_high();
    let mut high = config.high();

    let mut changed = false;

    egui::Grid::new(ui.id().with("deadzone_sliders"))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            changed |= drag_row(ui, "Low", &mut low);
            changed |= drag_row(ui, "Center Low", &mut center_low);
            changed |= drag_row(ui, "Center High", &mut center_high);
            changed |= drag_row(ui, "High", &mut high);
        });

    if changed {
        // Validate the new configuration; revert on error.
        DeadzoneConfig::new(low, center_low, center_high, high).ok()
    } else {
        None
    }
}

/// Map a normalized value in [-1, 1] to an x pixel coordinate within `rect`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "pixel coordinates are always within f32 range"
)]
fn norm_to_x(rect: &Rect, value: f64) -> f32 {
    rect.left() + ((value + 1.0) * 0.5) as f32 * rect.width()
}

/// Paint the custom deadzone visualization bar.
///
/// Shows five zones with distinct coloring:
///
/// - Below `low`: `COLOR_ERROR` (saturated)
/// - [`low`, `center_low`]: gradient blend (active negative zone)
/// - [`center_low`, `center_high`]: `COLOR_SURFACE0` with "DEAD" label
/// - [`center_high`, `high`]: gradient blend (active positive zone)
/// - Above `high`: `COLOR_ERROR` (saturated)
fn paint_deadzone_bar(ui: &mut egui::Ui, config: &DeadzoneConfig, live_input: Option<f64>) {
    let colors = theme::colors(ui.ctx());
    let desired_size = Vec2::new(BAR_WIDTH, BAR_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);

    let x_low = norm_to_x(&rect, config.low());
    let x_center_low = norm_to_x(&rect, config.center_low());
    let x_center_high = norm_to_x(&rect, config.center_high());
    let x_high = norm_to_x(&rect, config.high());

    // Zone 1: below low (saturated).
    if x_low > rect.left() {
        let zone = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(x_low, rect.bottom()),
        );
        painter.rect_filled(zone, 0.0, colors.zone_saturated());
    }

    // Zone 2: [low, center_low] (active negative zone).
    if x_center_low > x_low {
        let zone = Rect::from_min_max(
            Pos2::new(x_low, rect.top()),
            Pos2::new(x_center_low, rect.bottom()),
        );
        painter.rect_filled(zone, 0.0, colors.zone_negative());
    }

    // Zone 3: [center_low, center_high] (dead zone).
    if x_center_high >= x_center_low {
        let zone = Rect::from_min_max(
            Pos2::new(x_center_low, rect.top()),
            Pos2::new(x_center_high, rect.bottom()),
        );
        painter.rect_filled(zone, 0.0, colors.surface0);

        // "DEAD" label centered in the dead zone (only if wide enough).
        if (x_center_high - x_center_low) > 20.0 {
            let center = zone.center();
            let font = FontId::new(10.0, FontFamily::Monospace);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "DEAD",
                font,
                colors.text,
            );
        }
    }

    // Zone 4: [center_high, high] (active positive zone).
    if x_high > x_center_high {
        let zone = Rect::from_min_max(
            Pos2::new(x_center_high, rect.top()),
            Pos2::new(x_high, rect.bottom()),
        );
        painter.rect_filled(zone, 0.0, colors.zone_positive());
    }

    // Zone 5: above high (saturated).
    if x_high < rect.right() {
        let zone = Rect::from_min_max(
            Pos2::new(x_high, rect.top()),
            Pos2::new(rect.right(), rect.bottom()),
        );
        painter.rect_filled(zone, 0.0, colors.zone_saturated());
    }

    // Threshold markers (thin vertical lines).
    let marker_stroke = Stroke::new(1.0, colors.text_dim);
    for &x in &[x_low, x_center_low, x_center_high, x_high] {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            marker_stroke,
        );
    }

    // Live input marker.
    if let Some(input) = live_input {
        let x_input = norm_to_x(&rect, input.clamp(-1.0, 1.0));
        painter.line_segment(
            [
                Pos2::new(x_input, rect.top()),
                Pos2::new(x_input, rect.bottom()),
            ],
            Stroke::new(2.0, colors.live),
        );
    }

    // Border around the entire bar.
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, colors.surface1),
        StrokeKind::Outside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_colors_are_distinct_and_visible() {
        let colors = theme::colors(&egui::Context::default());
        assert_ne!(colors.zone_negative(), colors.zone_positive());
        assert_ne!(colors.zone_negative(), egui::Color32::TRANSPARENT);
        assert_ne!(colors.zone_positive(), egui::Color32::TRANSPARENT);
    }

    #[test]
    fn default_config_values_are_valid() {
        let config = DeadzoneConfig::default();
        assert!(config.low() < config.center_low());
        assert!(config.center_low() <= config.center_high());
        assert!(config.center_high() < config.high());
    }

    #[test]
    fn bar_dimensions_are_positive() {
        assert!(BAR_WIDTH > 0.0);
        assert!(BAR_HEIGHT > 0.0);
    }

    #[test]
    fn drag_speed_is_reasonable() {
        assert!(DRAG_SPEED > 0.0);
        assert!(DRAG_SPEED < 1.0);
    }

    #[test]
    fn norm_to_x_maps_endpoints() {
        let rect = Rect::from_min_max(Pos2::new(10.0, 0.0), Pos2::new(210.0, 30.0));
        let x_neg = norm_to_x(&rect, -1.0);
        let x_pos = norm_to_x(&rect, 1.0);
        assert!((x_neg - 10.0).abs() < f32::EPSILON);
        assert!((x_pos - 210.0).abs() < f32::EPSILON);
    }

    #[test]
    fn norm_to_x_maps_center() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(200.0, 30.0));
        let x_center = norm_to_x(&rect, 0.0);
        assert!((x_center - 100.0).abs() < f32::EPSILON);
    }
}
