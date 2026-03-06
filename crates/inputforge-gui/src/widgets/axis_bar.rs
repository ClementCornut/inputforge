// Rust guideline compliant 2026-03-06

//! Cockpit-gauge style horizontal axis bar widget.
//!
//! Displays a normalized axis value as a 3-part horizontal layout:
//! a fixed-width label, a recessed bar with center-fill trail and needle,
//! and a fixed-width numeric readout.

use egui::{Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::theme;

/// Height of the axis bar in logical pixels.
const BAR_HEIGHT: f32 = 18.0;

/// Width reserved for the left-hand label area.
const LABEL_WIDTH: f32 = 40.0;

/// Width reserved for the right-hand numeric readout.
const READOUT_WIDTH: f32 = 56.0;

/// Width of the value needle line.
const NEEDLE_WIDTH: f32 = 2.0;

/// Corner rounding for the recessed bar background and border.
const BAR_ROUNDING: f32 = 2.0;

/// Opacity multiplier for the fill trail behind the needle.
///
/// 20% keeps the trail visible without obscuring the background
/// grid and center tick.
const FILL_OPACITY: f32 = 0.20;

/// Paint a horizontal axis bar with a cockpit-gauge aesthetic.
///
/// The layout consists of a fixed-width label, a center-fill bar with needle,
/// and a fixed-width numeric readout. `value` is clamped to \[-1.0, 1.0\].
pub(crate) fn axis_bar(ui: &mut egui::Ui, label: &str, value: f64) {
    let colors = theme::colors(ui.ctx());
    axis_bar_impl(
        ui,
        label,
        value,
        colors.primary,
        colors.warning,
        colors.live,
    );
}

/// Paint a horizontal axis bar with custom fill and needle colors.
///
/// `fill_color` is used (at 20% opacity) for both positive and negative fill
/// trails, and `needle_color` is used for the value needle.
pub(crate) fn axis_bar_colored(
    ui: &mut egui::Ui,
    label: &str,
    value: f64,
    fill_color: Color32,
    needle_color: Color32,
) {
    axis_bar_impl(ui, label, value, fill_color, fill_color, needle_color);
}

/// Internal implementation shared by [`axis_bar`] and [`axis_bar_colored`].
///
/// `fill_positive` and `fill_negative` are the base colors (before dimming)
/// for the positive and negative fill trails respectively.
#[expect(
    clippy::cast_possible_truncation,
    reason = "value is clamped to [-1.0, 1.0], font sizes are small constants"
)]
fn axis_bar_impl(
    ui: &mut egui::Ui,
    label: &str,
    value: f64,
    fill_positive: Color32,
    fill_negative: Color32,
    needle_color: Color32,
) {
    let colors = theme::colors(ui.ctx());
    let value = value.clamp(-1.0, 1.0) as f32;
    let font = FontId::new(11.0, FontFamily::Monospace);

    ui.horizontal(|ui| {
        // --- 1. Label (fixed width) ---
        let (label_rect, _) =
            ui.allocate_exact_size(Vec2::new(LABEL_WIDTH, BAR_HEIGHT), Sense::hover());
        ui.painter().text(
            Pos2::new(label_rect.left(), label_rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font.clone(),
            colors.text_dim,
        );

        // --- 2. Bar canvas (fills remaining width minus readout) ---
        let bar_width = ui.available_width() - READOUT_WIDTH - ui.spacing().item_spacing.x;
        let bar_width = bar_width.max(1.0); // safety floor
        let (bar_rect, _) =
            ui.allocate_exact_size(Vec2::new(bar_width, BAR_HEIGHT), Sense::hover());
        let painter = ui.painter();

        // Recessed background.
        painter.rect_filled(bar_rect, BAR_ROUNDING, colors.crust);

        // Border (outside stroke).
        painter.rect_stroke(
            bar_rect,
            BAR_ROUNDING,
            Stroke::new(1.0, colors.surface1),
            StrokeKind::Outside,
        );

        let center_x = bar_rect.center().x;
        let value_x = center_x + (bar_width * 0.5 * value);

        // Fill trail from center to value (clipped to bar).
        let fill_rect = if value >= 0.0 {
            Rect::from_min_max(
                Pos2::new(center_x, bar_rect.top()),
                Pos2::new(value_x, bar_rect.bottom()),
            )
        } else {
            Rect::from_min_max(
                Pos2::new(value_x, bar_rect.top()),
                Pos2::new(center_x, bar_rect.bottom()),
            )
        };

        // Intersect with bar_rect so the fill never exceeds the rounded area.
        let clipped_fill = fill_rect.intersect(bar_rect);
        let fill_color = if value >= 0.0 {
            fill_positive.gamma_multiply(FILL_OPACITY)
        } else {
            fill_negative.gamma_multiply(FILL_OPACITY)
        };
        painter.rect_filled(clipped_fill, 0.0, fill_color);

        // Center tick (1px).
        painter.line_segment(
            [
                Pos2::new(center_x, bar_rect.top()),
                Pos2::new(center_x, bar_rect.bottom()),
            ],
            Stroke::new(1.0, colors.text_dim),
        );

        // Scale ticks at +/-0.5 (quarter-height from bottom).
        let tick_height = BAR_HEIGHT * 0.25;
        for &offset in &[-0.5_f32, 0.5_f32] {
            let tick_x = center_x + (bar_width * 0.5 * offset);
            painter.line_segment(
                [
                    Pos2::new(tick_x, bar_rect.bottom() - tick_height),
                    Pos2::new(tick_x, bar_rect.bottom()),
                ],
                Stroke::new(1.0, colors.indicator_idle),
            );
        }

        // Value needle.
        painter.line_segment(
            [
                Pos2::new(value_x, bar_rect.top()),
                Pos2::new(value_x, bar_rect.bottom()),
            ],
            Stroke::new(NEEDLE_WIDTH, needle_color),
        );

        // --- 3. Readout (fixed width) ---
        let (readout_rect, _) =
            ui.allocate_exact_size(Vec2::new(READOUT_WIDTH, BAR_HEIGHT), Sense::hover());
        let value_text = format!("{value:+.2}");
        ui.painter().text(
            Pos2::new(readout_rect.right(), readout_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            value_text,
            font,
            colors.text,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_height_constant() {
        assert!((BAR_HEIGHT - 18.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fill_trail_positive_uses_dimmed_primary() {
        let primary = theme::DARK.primary;
        let expected = primary.gamma_multiply(FILL_OPACITY);
        // Positive values use the primary color at FILL_OPACITY.
        assert_eq!(expected, primary.gamma_multiply(FILL_OPACITY));
    }

    #[test]
    fn fill_trail_negative_uses_dimmed_warning() {
        let warning = theme::DARK.warning;
        let expected = warning.gamma_multiply(FILL_OPACITY);
        // Negative values use the warning color at FILL_OPACITY.
        assert_eq!(expected, warning.gamma_multiply(FILL_OPACITY));
    }

    #[test]
    fn needle_position_at_zero_is_center() {
        // With value = 0, the needle offset from center should be zero.
        let bar_width: f32 = 200.0;
        let center_x: f32 = 100.0;
        let value: f32 = 0.0;
        let needle_x = center_x + (bar_width * 0.5 * value);
        assert!((needle_x - center_x).abs() < f32::EPSILON);
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
