// Rust guideline compliant 2026-03-06

//! Horizontal tab bar with animated sliding underline.
//!
//! Renders a row of text labels where exactly one is active. The active
//! tab is highlighted with the primary accent color and a semibold font
//! weight. A 2 px underline smoothly slides between tabs using egui's
//! built-in value animation.
//!
//! Two entry points are provided:
//! - [`tab_bar`], low-level, index-based API for maximum flexibility.
//! - [`tab_bar_enum`], convenience wrapper for enums implementing [`TabItem`].

use egui::FontFamily;

use crate::theme;

/// Underline stroke thickness in logical pixels.
const UNDERLINE_THICKNESS: f32 = 2.0;

/// Full-width separator stroke thickness in logical pixels.
const SEPARATOR_THICKNESS: f32 = 1.0;

/// Slide duration for the underline animation in seconds.
///
/// 150 ms feels snappy without being instant; egui handles repaint
/// requests automatically while the value is interpolating.
const UNDERLINE_ANIMATION_SECS: f32 = 0.15;

/// Trait for types that can be displayed as tab labels.
pub(crate) trait TabItem: Copy + PartialEq {
    /// Short display label shown in the tab bar.
    fn label(self) -> &'static str;
}

/// Render a horizontal tab bar and return the index of a newly clicked tab.
///
/// All tabs share an identical frameless shape, only text color and the
/// sliding underline distinguish the active tab from inactive ones.
///
/// `id_salt` must be unique per tab bar instance to keep animation state
/// independent when multiple tab bars coexist in the same frame.
pub(crate) fn tab_bar<'a>(
    ui: &mut egui::Ui,
    id_salt: &str,
    labels: impl IntoIterator<Item = (usize, &'a str)>,
    active: usize,
) -> Option<usize> {
    let colors = theme::colors(ui.ctx());
    let mut clicked = None;
    let mut tab_rects: Vec<(usize, egui::Rect)> = Vec::new();

    // --- Tab row -----------------------------------------------------------
    let row = ui.horizontal(|ui| {
        let button_padding = ui.spacing().button_padding;

        for (idx, label) in labels {
            let is_active = idx == active;

            // Pre-measure using semibold so all tabs reserve the wider width.
            let semibold_text =
                egui::RichText::new(label).family(FontFamily::Name("SemiBold".into()));
            let semibold_galley = egui::WidgetText::from(semibold_text).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Body,
            );
            let min_size = egui::vec2(semibold_galley.size().x + button_padding.x * 2.0, 0.0);

            let text = if is_active {
                egui::RichText::new(label)
                    .family(FontFamily::Name("SemiBold".into()))
                    .color(colors.primary)
            } else {
                egui::RichText::new(label).color(colors.text_dim)
            };

            let button = egui::Button::new(text).frame(false).min_size(min_size);
            let response = ui.add(button);

            // Re-render with brighter text on hover for inactive tabs.
            // Paint over the dimmed text first to avoid ClearType artifacts
            // from overlapping glyphs.
            if !is_active && response.hovered() {
                ui.painter()
                    .rect_filled(response.rect, 0.0, ui.visuals().panel_fill);
                ui.painter().text(
                    response.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(ui.style().text_styles[&egui::TextStyle::Body].size),
                    colors.text,
                );
            }

            if response.clicked() {
                clicked = Some(idx);
            }

            tab_rects.push((idx, response.rect));
        }
    });

    // --- Animated underline ------------------------------------------------
    let active_rect = tab_rects
        .iter()
        .find(|(idx, _)| *idx == active)
        .map(|(_, rect)| *rect);

    if let Some(target_rect) = active_rect {
        let ctx = ui.ctx();
        let base_id = ui.id().with(id_salt);
        let pad_x = ui.spacing().button_padding.x;

        // Inset by button padding so the underline spans only the text.
        let anim_x = ctx.animate_value_with_time(
            base_id.with("x"),
            target_rect.left() + pad_x,
            UNDERLINE_ANIMATION_SECS,
        );
        let anim_w = ctx.animate_value_with_time(
            base_id.with("w"),
            target_rect.width() - pad_x * 2.0,
            UNDERLINE_ANIMATION_SECS,
        );

        let separator_y = row.response.rect.bottom();
        let full_left = ui.max_rect().left();
        let full_right = ui.max_rect().right();

        // Full-width separator.
        ui.painter().hline(
            full_left..=full_right,
            separator_y,
            egui::Stroke::new(SEPARATOR_THICKNESS, colors.surface1),
        );

        // Accent underline at animated position.
        ui.painter().hline(
            anim_x..=(anim_x + anim_w),
            separator_y,
            egui::Stroke::new(UNDERLINE_THICKNESS, colors.primary),
        );
    }

    clicked
}

/// Convenience wrapper that maps an enum slice to [`tab_bar`].
///
/// Mutates `active` in place when the user clicks a different tab.
pub(crate) fn tab_bar_enum<T: TabItem>(
    ui: &mut egui::Ui,
    id_salt: &str,
    items: &[T],
    active: &mut T,
) {
    let active_idx = items.iter().position(|v| *v == *active).unwrap_or_else(|| {
        tracing::warn!("active tab not found in items, defaulting to index 0");
        0
    });
    let labels = items.iter().enumerate().map(|(i, v)| (i, v.label()));

    if let Some(clicked_idx) = tab_bar(ui, id_salt, labels, active_idx) {
        if let Some(item) = items.get(clicked_idx) {
            *active = *item;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const _: () = assert!(UNDERLINE_THICKNESS > 0.0);
    const _: () = assert!(SEPARATOR_THICKNESS > 0.0);
    const _: () = assert!(UNDERLINE_ANIMATION_SECS > 0.0);
    const _: () = assert!(UNDERLINE_ANIMATION_SECS >= 0.05);
    const _: () = assert!(UNDERLINE_ANIMATION_SECS <= 1.0);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestTab {
        Alpha,
        Beta,
    }

    impl TabItem for TestTab {
        fn label(self) -> &'static str {
            match self {
                Self::Alpha => "Alpha",
                Self::Beta => "Beta",
            }
        }
    }

    #[test]
    fn tab_item_labels_are_distinct() {
        assert_ne!(TestTab::Alpha.label(), TestTab::Beta.label());
    }
}
