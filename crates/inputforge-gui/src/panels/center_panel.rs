// Rust guideline compliant 2026-03-07

//! Center panel routing between application views.
//!
//! Displays a unified toolbar at the top: tab buttons on the left for
//! switching between two views (Mappings, Modes) and tool buttons on the
//! right (Input Viewer, Calibration, Profiles). The active tab determines
//! which content is rendered below.

use std::sync::mpsc;

use inputforge_core::engine::EngineCommand;

use crate::app::{CachedState, CenterView, GuiSelection, ToolWindowStates};
use crate::panels::calibration_window;
use crate::panels::input_viewer_window;
use crate::panels::mapping_editor::{self, MappingEditorState};
use crate::panels::profile_window;
use crate::theme;
use crate::widgets::empty_state;
use crate::widgets::tab_bar;

/// Render the center panel with unified toolbar and routed content.
///
/// This panel must be added LAST in `update()` because `CentralPanel`
/// fills remaining space after all side/bottom panels are placed.
///
/// Returns `true` when the user clicks "Discard" in the mapping editor,
/// signalling that the caller should reload the current mapping from the
/// saved profile.
#[must_use]
pub(crate) fn show(
    ctx: &egui::Context,
    cache: &CachedState,
    selection: &mut GuiSelection,
    mapping_editor_state: &mut MappingEditorState,
    tool_windows: &mut ToolWindowStates,
    commands: &mpsc::Sender<EngineCommand>,
) -> bool {
    let mut discard_requested = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        // Unified toolbar: tabs on the left, tool buttons on the right.
        ui.horizontal(|ui| {
            tab_bar::tab_bar_enum(
                ui,
                "center_tabs",
                &CenterView::all(),
                &mut selection.center_view,
            );

            show_tool_buttons(ui, ctx, tool_windows);
        });

        ui.add_space(4.0);

        // Routed content.
        match selection.center_view {
            CenterView::MappingEditor => {
                if mapping_editor::show(ui, mapping_editor_state, cache, commands) {
                    discard_requested = true;
                }
            }
            CenterView::ModeEditor => {
                show_mode_editor_stub(ui);
            }
        }
    });

    discard_requested
}

/// Render tool buttons right-aligned in the toolbar row.
fn show_tool_buttons(ui: &mut egui::Ui, ctx: &egui::Context, tool_windows: &mut ToolWindowStates) {
    let colors = theme::colors(ctx);

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let profiles_btn = ui.add(
            egui::Button::new(egui::RichText::new("Profiles").color(colors.text_dim)).frame(false),
        );
        if profiles_btn.hovered() {
            ui.painter().text(
                profiles_btn.rect.center(),
                egui::Align2::CENTER_CENTER,
                "Profiles",
                egui::FontId::proportional(ui.style().text_styles[&egui::TextStyle::Body].size),
                colors.text,
            );
        }
        if profiles_btn.clicked() {
            if tool_windows.profiles_open {
                ctx.send_viewport_cmd_to(
                    profile_window::viewport_id(),
                    egui::ViewportCommand::Focus,
                );
            } else {
                tool_windows.profiles_open = true;
            }
        }

        let btn = ui.add(
            egui::Button::new(egui::RichText::new("Calibration").color(colors.text_dim))
                .frame(false),
        );
        // Brighten text on hover (same pattern as tab_bar inactive tabs).
        if btn.hovered() {
            ui.painter().text(
                btn.rect.center(),
                egui::Align2::CENTER_CENTER,
                "Calibration",
                egui::FontId::proportional(ui.style().text_styles[&egui::TextStyle::Body].size),
                colors.text,
            );
        }
        if btn.clicked() {
            if tool_windows.calibration_open {
                ctx.send_viewport_cmd_to(
                    calibration_window::viewport_id(),
                    egui::ViewportCommand::Focus,
                );
            } else {
                tool_windows.calibration_open = true;
            }
        }

        let viewer_btn = ui.add(
            egui::Button::new(egui::RichText::new("Input Viewer").color(colors.text_dim))
                .frame(false),
        );
        if viewer_btn.hovered() {
            ui.painter().text(
                viewer_btn.rect.center(),
                egui::Align2::CENTER_CENTER,
                "Input Viewer",
                egui::FontId::proportional(ui.style().text_styles[&egui::TextStyle::Body].size),
                colors.text,
            );
        }
        if viewer_btn.clicked() {
            if tool_windows.input_viewer_open {
                ctx.send_viewport_cmd_to(
                    input_viewer_window::viewport_id(),
                    egui::ViewportCommand::Focus,
                );
            } else {
                tool_windows.input_viewer_open = true;
            }
        }
    });
}

/// Stub for the mode editor (implemented in Task 25).
fn show_mode_editor_stub(ui: &mut egui::Ui) {
    empty_state::empty_state(ui, "Mode editor \u{2014} coming soon");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_views_are_distinct() {
        let views = [CenterView::MappingEditor, CenterView::ModeEditor];
        for (i, a) in views.iter().enumerate() {
            for (j, b) in views.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
