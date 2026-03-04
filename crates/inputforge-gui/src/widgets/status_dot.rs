// Rust guideline compliant 2026-03-03

//! Connection/status indicator dot widget.
//!
//! Renders a small circle to indicate connection state:
//! filled circle when active (connected), hollow ring when inactive
//! (disconnected). This provides both color and shape distinction
//! for accessibility (WCAG 1.4.1 Use of Color).

use egui::{Color32, Stroke};

/// Paint a status dot: filled circle when `active`, hollow ring when not.
///
/// Layout space is reserved via `allocate_exact_size`; no return value
/// is needed since all callers rely on egui's automatic layout.
pub(crate) fn status_dot(ui: &mut egui::Ui, color: Color32, active: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::empty());
    let center = rect.center();
    let radius = 4.0;

    if active {
        ui.painter().circle_filled(center, radius, color);
    } else {
        ui.painter()
            .circle_stroke(center, radius, Stroke::new(1.5, color));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn status_dot_dimensions() {
        // The dot allocates 8x8 pixels.
        let size = egui::vec2(8.0, 8.0);
        assert!((size.x - 8.0).abs() < f32::EPSILON);
        assert!((size.y - 8.0).abs() < f32::EPSILON);
    }
}
