// Rust guideline compliant 2026-03-03

//! Resizable left sidebar panel displaying the device tree.
//!
//! Shows connected and disconnected devices as selectable entries
//! with a colored connection indicator dot. Clicking a device
//! selects it and switches the center panel to the device overview.

use crate::app::{CachedState, CenterView, GuiSelection};
use crate::theme;
use crate::widgets::{empty_state, status_dot};

/// Minimum width of the left panel in logical pixels.
const MIN_WIDTH: f32 = 240.0;

/// Maximum width of the left panel in logical pixels.
const MAX_WIDTH: f32 = 400.0;

/// Default width of the left panel in logical pixels.
const DEFAULT_WIDTH: f32 = 300.0;

/// Render the left sidebar panel with the device tree.
pub(crate) fn show(ctx: &egui::Context, cache: &CachedState, selection: &mut GuiSelection) {
    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(DEFAULT_WIDTH)
        .width_range(MIN_WIDTH..=MAX_WIDTH)
        .show(ctx, |ui| {
            if cache.devices.is_empty() {
                empty_state::empty_state(ui, "No devices detected");
                return;
            }

            let colors = theme::colors(ui.ctx());
            for (idx, device) in cache.devices.iter().enumerate() {
                let is_selected = selection.selected_device_idx == Some(idx);

                let dot_color = if device.connected {
                    colors.live
                } else {
                    colors.error
                };

                ui.horizontal(|ui| {
                    // Connection indicator: filled when connected, ring when not.
                    status_dot::status_dot(ui, dot_color, device.connected);

                    // Device name as selectable label. Disconnected devices
                    // are dimmed and italicized to match device_view.rs styling.
                    let label = if device.connected {
                        egui::RichText::new(&device.info.name)
                    } else {
                        egui::RichText::new(&device.info.name)
                            .color(colors.text_dim)
                            .italics()
                    };
                    let response = ui.selectable_label(is_selected, label);

                    if response.clicked() {
                        selection.selected_device_idx = Some(idx);
                        selection.center_view = CenterView::DeviceOverview;
                    }
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    const _: () = assert!(MIN_WIDTH < DEFAULT_WIDTH);
    const _: () = assert!(DEFAULT_WIDTH < MAX_WIDTH);
}
