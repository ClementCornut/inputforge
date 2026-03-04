// Rust guideline compliant 2026-03-03

//! Mode tree display and selection panel.
//!
//! Renders the hierarchical [`ModeTree`] as a collapsible tree view.
//! The currently active mode is highlighted with the live accent and bold text.
//! Clicking a mode node selects it for editing in the right-side editors.

use inputforge_core::mode::{ModeNode, ModeTree};

use crate::theme;
use crate::widgets::empty_state;

/// Render the mode tree panel.
///
/// - If `mode_tree` is `None` (no profile loaded), shows a placeholder message.
/// - `current_mode` is the engine's active mode, highlighted in green.
/// - `selected_mode` is the mode selected by the user for editing; updated on click.
#[expect(dead_code, reason = "called during integration in a later task")]
pub(crate) fn show(
    ui: &mut egui::Ui,
    mode_tree: Option<&ModeTree>,
    current_mode: &str,
    selected_mode: &mut String,
) {
    let Some(tree) = mode_tree else {
        show_no_profile(ui);
        return;
    };

    let colors = theme::colors(ui.ctx());

    ui.add_space(4.0);
    ui.label(egui::RichText::new("Mode Tree").color(colors.text).strong());
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            show_node(ui, tree.root(), current_mode, selected_mode, true);
        });
}

/// Display a placeholder when no profile is loaded.
fn show_no_profile(ui: &mut egui::Ui) {
    empty_state::empty_state(ui, "No profile loaded");
}

/// Recursively render a mode tree node.
///
/// Branch nodes (those with children) are rendered as collapsible headers.
/// Leaf nodes are rendered as selectable labels. The root is expanded by
/// default; all other branches start collapsed.
fn show_node(
    ui: &mut egui::Ui,
    node: &ModeNode,
    current_mode: &str,
    selected_mode: &mut String,
    is_root: bool,
) {
    let colors = theme::colors(ui.ctx());
    let is_active = node.name() == current_mode;
    let is_selected = node.name() == selected_mode.as_str();

    if node.children().is_empty() {
        // Leaf node: selectable label.
        let text = mode_label_text(node.name(), is_active, colors);
        if ui.selectable_label(is_selected, text).clicked() {
            node.name().clone_into(selected_mode);
        }
    } else {
        // Branch node: collapsible header.
        let text = mode_label_text(node.name(), is_active, colors);
        let header_response = egui::CollapsingHeader::new(text)
            .default_open(is_root)
            .show(ui, |ui| {
                for child in node.children() {
                    show_node(ui, child, current_mode, selected_mode, false);
                }
            });

        // Double-click selects a branch node for editing without
        // conflicting with the single-click expand/collapse toggle.
        if header_response.header_response.double_clicked() {
            node.name().clone_into(selected_mode);
        }
    }
}

/// Build a styled `RichText` label for a mode node.
///
/// Active modes are displayed in the live accent with bold formatting.
/// Inactive modes use the standard text color.
fn mode_label_text(name: &str, is_active: bool, colors: &theme::ThemeColors) -> egui::RichText {
    if is_active {
        egui::RichText::new(name).color(colors.live).strong()
    } else {
        egui::RichText::new(name).color(colors.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_label_text_preserves_name_for_both_states() {
        let active = mode_label_text("Combat", true, &theme::DARK);
        assert_eq!(active.text(), "Combat");

        let inactive = mode_label_text("Landing", false, &theme::DARK);
        assert_eq!(inactive.text(), "Landing");
    }
}
