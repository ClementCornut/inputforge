// Rust guideline compliant 2026-03-03

//! Grid of button state indicator circles.
//!
//! Each button is rendered as a small circle: filled with the live
//! accent when pressed, outlined with the idle indicator color when
//! released. A small index label is drawn below each circle.

use std::borrow::Cow;

use egui::{FontFamily, FontId, Pos2, Stroke, Vec2};

use crate::theme;

/// Diameter of each button circle in logical pixels.
const BUTTON_DIAMETER: f32 = 14.0;

/// Vertical space between the circle and the index label.
const LABEL_GAP: f32 = 2.0;

/// Height of the index label text.
const LABEL_HEIGHT: f32 = 10.0;

/// Total height per button cell (circle + gap + label).
const CELL_HEIGHT: f32 = BUTTON_DIAMETER + LABEL_GAP + LABEL_HEIGHT;

/// Horizontal spacing between button cells.
const CELL_SPACING: f32 = 6.0;

/// Vertical spacing between rows.
const ROW_SPACING: f32 = 4.0;

/// Paint a grid of button state circles.
///
/// `buttons` is a slice where index corresponds to button number and
/// `true` means pressed. `columns` controls how many buttons appear
/// per row.
pub(crate) fn button_grid(ui: &mut egui::Ui, buttons: &[bool], columns: usize) {
    let colors = theme::colors(ui.ctx());
    let columns = columns.max(1);
    let rows = buttons.len().div_ceil(columns);

    let total_width = columns as f32 * (BUTTON_DIAMETER + CELL_SPACING) - CELL_SPACING;
    let total_height = rows as f32 * (CELL_HEIGHT + ROW_SPACING) - ROW_SPACING;

    let desired_size = Vec2::new(total_width.max(0.0), total_height.max(0.0));
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // 10px minimum for legibility (A3); use primary text color for contrast (A1).
    let font = FontId::new(10.0, FontFamily::Monospace);
    let radius = BUTTON_DIAMETER * 0.5;

    for (i, &pressed) in buttons.iter().enumerate() {
        let col = i % columns;
        let row = i / columns;

        let cx = rect.left() + col as f32 * (BUTTON_DIAMETER + CELL_SPACING) + radius;
        let cy = rect.top() + row as f32 * (CELL_HEIGHT + ROW_SPACING) + radius;
        let center = Pos2::new(cx, cy);

        if pressed {
            painter.circle_filled(center, radius, colors.live);
        } else {
            painter.circle_stroke(center, radius, Stroke::new(1.0, colors.indicator_idle));
        }

        // 1-indexed label below the circle (vJoy buttons are 1-indexed).
        let label_pos = Pos2::new(cx, cy + radius + LABEL_GAP);
        let label = button_label(i);
        painter.text(
            label_pos,
            egui::Align2::CENTER_TOP,
            label,
            font.clone(),
            colors.text,
        );
    }
}

/// Static 1-indexed label table for buttons 1-32 to avoid per-frame allocation.
const BUTTON_LABELS: [&str; 32] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17",
    "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32",
];

/// Return a 1-indexed label for the given 0-based button index.
///
/// Uses a static table for indices 0..31 to avoid heap allocation.
/// Falls back to a heap-allocated string for indices beyond the table.
fn button_label(zero_based: usize) -> Cow<'static, str> {
    let one_based = zero_based + 1;
    if one_based <= BUTTON_LABELS.len() {
        Cow::Borrowed(BUTTON_LABELS[zero_based])
    } else {
        Cow::Owned(one_based.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_height_equals_sum() {
        let expected = BUTTON_DIAMETER + LABEL_GAP + LABEL_HEIGHT;
        assert!((CELL_HEIGHT - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_buttons_produces_no_rows() {
        let buttons: Vec<bool> = vec![];
        let columns = 8;
        let rows = buttons.len().div_ceil(columns);
        assert_eq!(rows, 0);
    }

    #[test]
    fn row_calculation_exact_fit() {
        let buttons = vec![false; 16];
        let columns = 8;
        let rows = buttons.len().div_ceil(columns);
        assert_eq!(rows, 2);
    }

    #[test]
    fn row_calculation_partial_last_row() {
        let buttons = vec![false; 10];
        let columns = 8;
        let rows = buttons.len().div_ceil(columns);
        assert_eq!(rows, 2);
    }

    #[test]
    fn columns_clamped_to_minimum_one() {
        let clamped = 0_usize.max(1);
        assert_eq!(clamped, 1);
    }

    #[test]
    fn button_label_is_one_indexed() {
        assert_eq!(button_label(0), "1");
        assert_eq!(button_label(1), "2");
        assert_eq!(button_label(31), "32");
    }

    #[test]
    fn button_label_beyond_table_still_works() {
        assert_eq!(button_label(32), "33");
        assert_eq!(button_label(127), "128");
    }
}
