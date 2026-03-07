// Rust guideline compliant 2026-03-06

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

/// HID standard axis names for indices 0–7.
///
/// Maps to the standard HID usage page ordering: X, Y, Z, then rotational
/// axes, then slider and dial.  Uses abbreviated forms to fit the 40 px
/// label area.
pub(crate) const HID_AXIS_LABELS: [&str; 8] =
    ["X", "Y", "Z", "Rot X", "Rot Y", "Rot Z", "Sldr", "Dial"];

/// Return a human-readable label for the given 0-based axis index.
///
/// Indices 0–7 use HID standard names; higher indices fall back to
/// `Ax {index}`.
pub(crate) fn axis_label(index: usize) -> Cow<'static, str> {
    if index < HID_AXIS_LABELS.len() {
        Cow::Borrowed(HID_AXIS_LABELS[index])
    } else {
        Cow::Owned(format!("Ax {index}"))
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
                for (i, &(value, polarity)) in snapshot.axes.iter().enumerate() {
                    axis_bar::axis_bar(ui, &axis_label(i), value, polarity);
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
    info.name.clone()
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
                axis_polarities: vec![],
            },
            connected: true,
        }
    }

    #[test]
    fn build_header_text_format() {
        let device = sample_device();
        let text = build_header_text(&device);
        assert_eq!(text, "Test Joystick");
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
                axis_polarities: vec![],
            },
            connected: false,
        };
        let text = build_header_text(&device);
        assert_eq!(text, "Empty Device");
    }

    #[test]
    fn axis_label_hid_names() {
        assert_eq!(axis_label(0), "X");
        assert_eq!(axis_label(2), "Z");
        assert_eq!(axis_label(3), "Rot X");
        assert_eq!(axis_label(6), "Sldr");
        assert_eq!(axis_label(7), "Dial");
    }

    #[test]
    fn axis_label_beyond_table_falls_back() {
        assert_eq!(axis_label(8), "Ax 8");
        assert_eq!(axis_label(99), "Ax 99");
    }
}
