// Rust guideline compliant 2026-03-03

//! Center panel routing between application views.
//!
//! Displays a horizontal tab bar at the top for switching between
//! four views: Devices, Mappings, Monitor, and Modes. The active
//! tab determines which content is rendered below.

use crate::app::{CachedState, CenterView, GuiSelection};
use crate::panels::device_view;
use crate::panels::input_monitor::{self, InputMonitorState};
use crate::panels::mapping_editor::{self, MappingEditorState};
use crate::theme;
use crate::widgets::empty_state;

/// Render the center panel with tab bar and routed content.
///
/// This panel must be added LAST in `update()` because `CentralPanel`
/// fills remaining space after all side/bottom panels are placed.
pub(crate) fn show(
    ctx: &egui::Context,
    cache: &CachedState,
    selection: &mut GuiSelection,
    monitor_state: &mut InputMonitorState,
    mapping_editor_state: &mut MappingEditorState,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // Tab bar.
        ui.horizontal(|ui| {
            for view in CenterView::all() {
                tab_button(ui, view.label(), view, selection);
            }
        });

        ui.separator();

        // Routed content.
        match selection.center_view {
            CenterView::DeviceOverview => {
                device_view::show(ui, cache);
            }
            CenterView::MappingEditor => {
                mapping_editor::show(ui, mapping_editor_state, cache, selection);
            }
            CenterView::InputMonitor => {
                input_monitor::show(ui, monitor_state);
            }
            CenterView::ModeEditor => {
                show_mode_editor_stub(ui);
            }
        }
    });
}

/// Render a single tab button with active highlighting.
fn tab_button(ui: &mut egui::Ui, label: &str, view: CenterView, selection: &mut GuiSelection) {
    let colors = theme::colors(ui.ctx());
    let is_active = selection.center_view == view;

    let text = if is_active {
        egui::RichText::new(label).color(colors.primary)
    } else {
        egui::RichText::new(label).color(colors.text_dim)
    };

    // Give inactive tabs a subtle background so they look clickable.
    let button = if is_active {
        egui::Button::new(text).frame(false)
    } else {
        egui::Button::new(text).fill(colors.surface0)
    };
    let response = ui.add(button);

    if response.clicked() {
        selection.center_view = view;
    }

    // Active indicator underline drawn under this button's rect.
    if is_active {
        let rect = response.rect;
        ui.painter().hline(
            rect.left()..=rect.right(),
            rect.bottom(),
            egui::Stroke::new(2.0, colors.primary),
        );
    }
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
        let views = [
            CenterView::DeviceOverview,
            CenterView::MappingEditor,
            CenterView::InputMonitor,
            CenterView::ModeEditor,
        ];
        for (i, a) in views.iter().enumerate() {
            for (j, b) in views.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
