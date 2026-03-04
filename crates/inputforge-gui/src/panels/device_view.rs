// Rust guideline compliant 2026-03-03

//! Device overview panel showing all connected devices with live inputs.
//!
//! For each device a collapsing header displays the device name and
//! connection status. Inside, axis bars, a button grid, and hat
//! indicators render live values read from the shared [`AppState`]
//! input cache.

use std::borrow::Cow;

use egui::FontFamily;
use inputforge_core::state::DeviceState;

use crate::app::{CachedState, DeviceInputSnapshot};
use crate::theme;
use crate::widgets::{axis_bar, button_grid, empty_state, hat_indicator, status_dot};

/// Static 0-indexed axis label table for axes 0-15 to avoid per-frame allocation.
const AXIS_LABELS: [&str; 16] = [
    "A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "A10", "A11", "A12", "A13", "A14",
    "A15",
];

/// Return a label for the given 0-based axis index.
///
/// Uses a static table for indices 0-15 to avoid heap allocation.
fn axis_label(index: usize) -> Cow<'static, str> {
    if index < AXIS_LABELS.len() {
        Cow::Borrowed(AXIS_LABELS[index])
    } else {
        Cow::Owned(format!("A{index}"))
    }
}

/// Render the device overview panel.
///
/// Iterates over all cached devices and renders a collapsible section
/// for each one containing live axis, button, and hat state.
///
/// All input values are pre-snapshotted in `CachedState::input_snapshots`
/// under a single lock — no lock acquisitions happen here.
pub(crate) fn show(ui: &mut egui::Ui, cache: &CachedState) {
    if cache.devices.is_empty() {
        empty_state::empty_state(ui, "No devices detected");
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for (device, snapshot) in cache.devices.iter().zip(&cache.input_snapshots) {
                show_device(ui, device, snapshot);
            }
        });
}

/// Render a single device section with collapsing header and live inputs.
fn show_device(ui: &mut egui::Ui, device: &DeviceState, snapshot: &DeviceInputSnapshot) {
    let colors = theme::colors(ui.ctx());
    let header_text = build_header_text(device);

    let id = ui.make_persistent_id(&device.info.id.0);
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            // Connection indicator: filled when connected, ring when not.
            let dot_color = if device.connected {
                colors.live
            } else {
                colors.error
            };
            status_dot::status_dot(ui, dot_color, device.connected);

            ui.label(egui::RichText::new(&header_text).color(colors.text));
        })
        .body(|ui| {
            if !device.connected {
                ui.label(
                    egui::RichText::new("Disconnected")
                        .color(colors.error)
                        .italics(),
                );
                return;
            }

            // Axes section.
            if !snapshot.axes.is_empty() {
                ui.label(
                    egui::RichText::new("Axes")
                        .color(colors.text)
                        .family(FontFamily::Name("SemiBold".into())),
                );
                for (i, &value) in snapshot.axes.iter().enumerate() {
                    axis_bar::axis_bar(ui, &axis_label(i), value);
                    ui.add_space(1.0);
                }
                ui.add_space(4.0);
            }

            // Buttons section.
            if !snapshot.buttons.is_empty() {
                ui.label(
                    egui::RichText::new("Buttons")
                        .color(colors.text)
                        .family(FontFamily::Name("SemiBold".into())),
                );
                button_grid::button_grid(ui, &snapshot.buttons, 8);
                ui.add_space(4.0);
            }

            // Hats section.
            if !snapshot.hats.is_empty() {
                ui.label(
                    egui::RichText::new("Hats")
                        .color(colors.text)
                        .family(FontFamily::Name("SemiBold".into())),
                );
                ui.horizontal(|ui| {
                    for &dir in &snapshot.hats {
                        hat_indicator::hat_indicator(ui, dir)
                            .on_hover_text(hat_indicator::direction_label(dir));
                        ui.add_space(4.0);
                    }
                });
            }
        });
}

/// Build a header string from device info: "Name (Xa, Yb, Zh)".
fn build_header_text(device: &DeviceState) -> String {
    let info = &device.info;
    format!(
        "{} ({}a, {}b, {}h)",
        info.name, info.axes, info.buttons, info.hats
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, DeviceInfo};

    fn sample_device() -> DeviceState {
        DeviceState {
            info: DeviceInfo {
                id: DeviceId("test-dev".to_owned()),
                name: "Test Joystick".to_owned(),
                axes: 3,
                buttons: 12,
                hats: 1,
                instance_path: None,
            },
            connected: true,
        }
    }

    #[test]
    fn build_header_text_format() {
        let device = sample_device();
        let text = build_header_text(&device);
        assert_eq!(text, "Test Joystick (3a, 12b, 1h)");
    }

    #[test]
    fn build_header_text_zero_inputs() {
        let device = DeviceState {
            info: DeviceInfo {
                id: DeviceId("empty".to_owned()),
                name: "Empty Device".to_owned(),
                axes: 0,
                buttons: 0,
                hats: 0,
                instance_path: None,
            },
            connected: false,
        };
        let text = build_header_text(&device);
        assert_eq!(text, "Empty Device (0a, 0b, 0h)");
    }

    #[test]
    fn axis_label_uses_static_table() {
        assert_eq!(axis_label(0), "A0");
        assert_eq!(axis_label(7), "A7");
        assert_eq!(axis_label(15), "A15");
    }

    #[test]
    fn axis_label_beyond_table_falls_back() {
        assert_eq!(axis_label(16), "A16");
        assert_eq!(axis_label(99), "A99");
    }
}
