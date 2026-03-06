// Rust guideline compliant 2026-03-04

//! Visual calibration bar widget.
//!
//! Renders a custom-painted horizontal bar showing the physical-to-normalized
//! mapping with threshold markers and optional live input indicator.

use egui::{FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

use inputforge_core::processing::calibration::Calibration;

use crate::theme;

/// Width of the visual preview bar in logical pixels.
const BAR_WIDTH: f32 = 200.0;

/// Height of the visual preview bar in logical pixels.
const BAR_HEIGHT: f32 = 30.0;

/// Map a physical value to an x pixel coordinate within `rect`,
/// given the display range [`display_min`, `display_max`].
#[expect(
    clippy::cast_possible_truncation,
    reason = "pixel coordinates are always within f32 range"
)]
fn physical_to_x(rect: &Rect, value: f64, display_min: f64, display_range: f64) -> f32 {
    rect.left() + ((value - display_min) / display_range) as f32 * rect.width()
}

/// Paint the custom calibration visualization bar.
///
/// Shows the physical range divided into zones:
///
/// - `[min, center_low]`: negative active zone (maps to `[-1, 0]`)
/// - `[center_low, center_high]`: center band (maps to `0`)
/// - `[center_high, max]`: positive active zone (maps to `[0, 1]`)
///
/// With threshold markers and optional live input indicator.
pub(crate) fn paint_calibration_bar(
    ui: &mut egui::Ui,
    config: &Calibration,
    live_input: Option<f64>,
) {
    let colors = theme::colors(ui.ctx());
    let desired_size = Vec2::new(BAR_WIDTH, BAR_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);

    // Background fill.
    painter.rect_filled(rect, 0.0, colors.surface0);

    // Determine the display range from the calibration thresholds.
    let display_min = config.physical_min();
    let display_max = config.physical_max();
    let display_range = display_max - display_min;

    // Guard against zero-width range.
    if display_range <= 0.0 {
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, colors.surface1),
            StrokeKind::Outside,
        );
        return;
    }

    let to_x = |v: f64| -> f32 { physical_to_x(&rect, v, display_min, display_range) };

    let x_phys_min = to_x(config.physical_min());
    let x_center_low = to_x(config.physical_center_low());
    let x_center_high = to_x(config.physical_center_high());
    let x_phys_max = to_x(config.physical_max());

    // Negative active zone: [min, center_low].
    if x_center_low > x_phys_min {
        let zone = Rect::from_min_max(
            Pos2::new(x_phys_min, rect.top()),
            Pos2::new(x_center_low, rect.bottom()),
        );
        painter.rect_filled(zone, 0.0, colors.zone_negative());
    }

    // Center band: [center_low, center_high].
    if x_center_high >= x_center_low {
        let zone = Rect::from_min_max(
            Pos2::new(x_center_low, rect.top()),
            Pos2::new(x_center_high, rect.bottom()),
        );
        painter.rect_filled(zone, 0.0, colors.surface1);

        // "CTR" label (only if wide enough).
        if (x_center_high - x_center_low) > 30.0 {
            let center = zone.center();
            let font = FontId::new(10.0, FontFamily::Monospace);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "CTR",
                font,
                colors.text,
            );
        }
    }

    // Positive active zone: [center_high, max].
    if x_phys_max > x_center_high {
        let zone = Rect::from_min_max(
            Pos2::new(x_center_high, rect.top()),
            Pos2::new(x_phys_max, rect.bottom()),
        );
        painter.rect_filled(zone, 0.0, colors.zone_positive());
    }

    // Threshold markers.
    let marker_stroke = Stroke::new(1.0, colors.text_dim);
    for &x in &[x_phys_min, x_center_low, x_center_high, x_phys_max] {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            marker_stroke,
        );
    }

    // Marker labels at the bottom edge (10px minimum for legibility).
    let label_font = FontId::new(10.0, FontFamily::Monospace);
    let labels = [
        (x_phys_min, "min"),
        (x_center_low, "cL"),
        (x_center_high, "cH"),
        (x_phys_max, "max"),
    ];
    for (x, label) in labels {
        painter.text(
            Pos2::new(x, rect.bottom() - 2.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            label_font.clone(),
            colors.text_dim,
        );
    }

    // Live input marker.
    if let Some(input) = live_input {
        let clamped = input.clamp(display_min, display_max);
        let x_input = to_x(clamped);
        painter.line_segment(
            [
                Pos2::new(x_input, rect.top()),
                Pos2::new(x_input, rect.bottom()),
            ],
            Stroke::new(2.0, colors.live),
        );
    }

    paint_bar_overlay_and_border(&painter, &rect, config.enabled(), colors);
}

/// Paint the disabled overlay (when calibration is off) and the border.
fn paint_bar_overlay_and_border(
    painter: &egui::Painter,
    rect: &Rect,
    enabled: bool,
    colors: &theme::ThemeColors,
) {
    if !enabled {
        painter.rect_filled(*rect, 0.0, colors.disabled_overlay());
        let font = FontId::new(12.0, FontFamily::Proportional);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "DISABLED",
            font,
            colors.text_dim,
        );
    }

    painter.rect_stroke(
        *rect,
        0.0,
        Stroke::new(1.0, colors.surface1),
        StrokeKind::Outside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_calibration() -> Calibration {
        Calibration::new(-1.0, -0.05, 0.05, 1.0, true).unwrap()
    }

    #[test]
    fn test_calibration_values_are_valid() {
        let cal = test_calibration();
        assert!(cal.physical_min() < cal.physical_center_low());
        assert!(cal.physical_center_low() <= cal.physical_center_high());
        assert!(cal.physical_center_high() < cal.physical_max());
    }

    #[test]
    fn bar_dimensions_are_positive() {
        assert!(BAR_WIDTH > 0.0);
        assert!(BAR_HEIGHT > 0.0);
    }

    #[test]
    fn disabled_calibration_passes_through() {
        let cal = Calibration::new(-1.0, -0.05, 0.05, 1.0, false).unwrap();
        assert!(!cal.enabled());
        // When disabled, apply() returns the raw value.
        assert!((cal.apply(0.42) - 0.42).abs() < f64::EPSILON);
    }

    #[test]
    fn physical_to_x_maps_endpoints() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(200.0, 30.0));
        let x_min = physical_to_x(&rect, -1.0, -1.0, 2.0);
        let x_max = physical_to_x(&rect, 1.0, -1.0, 2.0);
        assert!((x_min - 0.0).abs() < f32::EPSILON);
        assert!((x_max - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn physical_to_x_maps_center() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(200.0, 30.0));
        let x_center = physical_to_x(&rect, 0.0, -1.0, 2.0);
        assert!((x_center - 100.0).abs() < f32::EPSILON);
    }
}
