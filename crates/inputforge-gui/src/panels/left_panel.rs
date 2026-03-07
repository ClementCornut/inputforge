// Rust guideline compliant 2026-03-07

//! Resizable left sidebar panel displaying the device tree.
//!
//! Shows connected and disconnected devices as selectable entries
//! with a colored connection indicator dot. When a device is selected
//! and connected, its inputs are listed below grouped by type
//! (Axes, Buttons, Hats) with mapping indicators.

use inputforge_core::types::{InputAddress, InputId};

use crate::app::{CachedState, CenterView, GuiSelection};
use crate::panels::device_view;
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

            egui::ScrollArea::vertical().show(ui, |ui| {
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
                            selection.selected_input = None;
                            selection.center_view = CenterView::MappingEditor;
                        }
                    });

                    // Show input tree when device is selected and connected.
                    if is_selected && device.connected {
                        ui.indent("device_inputs", |ui| {
                            input_tree(ui, colors, cache, device, selection);
                        });
                    }
                }
            });
        });
}

/// Render the input tree (Axes / Buttons / Hats) for a selected device.
fn input_tree(
    ui: &mut egui::Ui,
    colors: &theme::ThemeColors,
    cache: &CachedState,
    device: &inputforge_core::state::DeviceState,
    selection: &mut GuiSelection,
) {
    // Axes section
    if device.info.axes > 0 {
        ui.label(egui::RichText::new("Axes").color(colors.text_dim).small());
        for i in 0..device.info.axes {
            let input_id = InputId::Axis { index: i };
            let addr = InputAddress {
                device: device.info.id.clone(),
                input: input_id.clone(),
            };
            let is_input_selected = selection.selected_input.as_ref() == Some(&input_id);
            let is_mapped = cache.mapped_inputs.contains(&addr);
            let mapping_name = cache.mapping_names.get(&addr);
            let label_text = device_view::axis_label(usize::from(i));

            input_row(
                ui,
                colors,
                is_input_selected,
                is_mapped,
                &label_text,
                mapping_name,
                || {
                    selection.selected_input = Some(input_id.clone());
                },
            );
        }
    }

    // Buttons section
    if device.info.buttons > 0 {
        ui.label(
            egui::RichText::new("Buttons")
                .color(colors.text_dim)
                .small(),
        );
        for i in 0..device.info.buttons {
            let input_id = InputId::Button { index: i };
            let addr = InputAddress {
                device: device.info.id.clone(),
                input: input_id.clone(),
            };
            let is_input_selected = selection.selected_input.as_ref() == Some(&input_id);
            let is_mapped = cache.mapped_inputs.contains(&addr);
            let mapping_name = cache.mapping_names.get(&addr);
            let label_text = format!("{}", i + 1);

            input_row(
                ui,
                colors,
                is_input_selected,
                is_mapped,
                &label_text,
                mapping_name,
                || {
                    selection.selected_input = Some(input_id.clone());
                },
            );
        }
    }

    // Hats section
    if device.info.hats > 0 {
        ui.label(egui::RichText::new("Hats").color(colors.text_dim).small());
        for i in 0..device.info.hats {
            let input_id = InputId::Hat { index: i };
            let addr = InputAddress {
                device: device.info.id.clone(),
                input: input_id.clone(),
            };
            let is_input_selected = selection.selected_input.as_ref() == Some(&input_id);
            let is_mapped = cache.mapped_inputs.contains(&addr);
            let mapping_name = cache.mapping_names.get(&addr);
            let label_text = format!("Hat {i}");

            input_row(
                ui,
                colors,
                is_input_selected,
                is_mapped,
                &label_text,
                mapping_name,
                || {
                    selection.selected_input = Some(input_id.clone());
                },
            );
        }
    }
}

/// Render a single input row with optional mapping indicator and name.
fn input_row(
    ui: &mut egui::Ui,
    colors: &theme::ThemeColors,
    is_selected: bool,
    is_mapped: bool,
    label: &str,
    mapping_name: Option<&String>,
    on_click: impl FnOnce(),
) {
    ui.horizontal(|ui| {
        if is_mapped {
            ui.label(egui::RichText::new("●").color(colors.primary).small());
        }

        let response = ui.selectable_label(is_selected, label);

        if let Some(name) = mapping_name {
            ui.label(
                egui::RichText::new(name)
                    .color(colors.text_dim)
                    .small()
                    .italics(),
            );
        }

        if response.clicked() {
            on_click();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const _: () = assert!(MIN_WIDTH < DEFAULT_WIDTH);
    const _: () = assert!(DEFAULT_WIDTH < MAX_WIDTH);
}
