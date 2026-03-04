// Rust guideline compliant 2026-03-03

//! Custom-painted horizontal axis bar widget.
//!
//! Displays a normalized axis value as a horizontal bar that fills from
//! the center: positive values fill right in the primary accent, negative
//! values fill left in a dimmed primary variant.

use egui::{Color32, FontFamily, FontId, Pos2, Rect, Stroke, Vec2};

use crate::theme;

/// Height of the axis bar in logical pixels.
const BAR_HEIGHT: f32 = 14.0;

/// Paint a horizontal axis bar with an inline label and value readout.
///
/// `value` is clamped to [-1.0, 1.0] before rendering.
pub(crate) fn axis_bar(ui: &mut egui::Ui, label: &str, value: f64) {
    let colors = theme::colors(ui.ctx());

    #[expect(
        clippy::cast_possible_truncation,
        reason = "value is clamped to [-1.0, 1.0]"
    )]
    let value = value.clamp(-1.0, 1.0) as f32;

    let available_width = ui.available_width();
    let desired_size = Vec2::new(available_width, BAR_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);

    // Background fill.
    painter.rect_filled(rect, 0.0, colors.surface0);

    // Center x coordinate.
    let center_x = rect.center().x;

    // Fill bar from center.
    let fill_color = fill_color_for_value(value, colors.primary);
    let fill_rect = if value >= 0.0 {
        Rect::from_min_max(
            Pos2::new(center_x, rect.top()),
            Pos2::new(center_x + (rect.width() * 0.5 * value), rect.bottom()),
        )
    } else {
        Rect::from_min_max(
            Pos2::new(center_x + (rect.width() * 0.5 * value), rect.top()),
            Pos2::new(center_x, rect.bottom()),
        )
    };
    painter.rect_filled(fill_rect, 0.0, fill_color);

    // Center tick line (1px).
    painter.line_segment(
        [
            Pos2::new(center_x, rect.top()),
            Pos2::new(center_x, rect.bottom()),
        ],
        Stroke::new(1.0, colors.surface1),
    );

    // Inline label on the left.
    let font = FontId::new(10.0, FontFamily::Monospace);
    let text_y = rect.center().y;
    painter.text(
        Pos2::new(rect.left() + 4.0, text_y),
        egui::Align2::LEFT_CENTER,
        label,
        font.clone(),
        colors.text,
    );

    // Value readout on the right.
    let value_text = format!("{value:+.2}");
    painter.text(
        Pos2::new(rect.right() - 4.0, text_y),
        egui::Align2::RIGHT_CENTER,
        value_text,
        font,
        colors.text_dim,
    );
}

/// Return the fill color based on axis value sign.
///
/// Positive values use the primary accent color. Negative values use a
/// dimmed variant (60% opacity) to distinguish direction without implying
/// a warning state.
fn fill_color_for_value(value: f32, primary: Color32) -> Color32 {
    if value >= 0.0 {
        primary
    } else {
        primary.gamma_multiply(0.6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_color_positive_is_primary() {
        let primary = theme::DARK.primary;
        assert_eq!(fill_color_for_value(0.5, primary), primary);
        assert_eq!(fill_color_for_value(0.0, primary), primary);
    }

    #[test]
    fn fill_color_negative_is_dimmed_primary() {
        let primary = theme::DARK.primary;
        let dimmed = primary.gamma_multiply(0.6);
        assert_eq!(fill_color_for_value(-0.5, primary), dimmed);
        assert_eq!(fill_color_for_value(-1.0, primary), dimmed);
    }

    #[test]
    fn value_clamp_positive() {
        let clamped = 1.5_f64.clamp(-1.0, 1.0);
        assert!((clamped - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn value_clamp_negative() {
        let clamped = (-2.0_f64).clamp(-1.0, 1.0);
        assert!((clamped - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn value_clamp_in_range() {
        let clamped = 0.5_f64.clamp(-1.0, 1.0);
        assert!((clamped - 0.5).abs() < f64::EPSILON);
    }
}
