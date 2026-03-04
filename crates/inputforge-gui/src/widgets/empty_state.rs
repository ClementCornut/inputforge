// Rust guideline compliant 2026-03-03

//! Reusable placeholder widget for empty or "coming soon" views.
//!
//! Renders centered, dimmed, italic text with consistent vertical spacing
//! across all panels that display placeholder content.

use crate::theme;

/// Vertical padding above the placeholder label in logical pixels.
const TOP_PADDING: f32 = 40.0;

/// Display a centered placeholder label for empty or unimplemented views.
///
/// Uses the dimmed text color with italic styling and a 40px top spacer
/// for visual consistency across all panels.
pub(crate) fn empty_state(ui: &mut egui::Ui, text: &str) {
    let colors = theme::colors(ui.ctx());
    ui.vertical_centered(|ui| {
        ui.add_space(TOP_PADDING);
        ui.label(egui::RichText::new(text).color(colors.text_dim).italics());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_padding_is_positive() {
        assert!(TOP_PADDING > 0.0);
    }
}
