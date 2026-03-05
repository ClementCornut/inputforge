// Rust guideline compliant 2026-03-04

//! vJoy output configuration UI.
//!
//! Renders device, output type, and axis/button/hat selectors for the
//! [`Action::MapToVJoy`] variant.

use inputforge_core::types::{OutputId, VJoyAxis, VirtualDeviceConfig};

use crate::theme::ThemeColors;

use super::description;

/// All [`VJoyAxis`] variants in UI display order.
const VJOY_AXES: [VJoyAxis; 8] = [
    VJoyAxis::X,
    VJoyAxis::Y,
    VJoyAxis::Z,
    VJoyAxis::Rx,
    VJoyAxis::Ry,
    VJoyAxis::Rz,
    VJoyAxis::Slider0,
    VJoyAxis::Slider1,
];

/// Display label for a [`VJoyAxis`] variant.
fn vjoy_axis_label(axis: VJoyAxis) -> &'static str {
    match axis {
        VJoyAxis::X => "X",
        VJoyAxis::Y => "Y",
        VJoyAxis::Z => "Z",
        VJoyAxis::Rx => "X Rotation",
        VJoyAxis::Ry => "Y Rotation",
        VJoyAxis::Rz => "Z Rotation",
        VJoyAxis::Slider0 => "Slider",
        VJoyAxis::Slider1 => "Dial",
    }
}

/// Human-readable label for an [`OutputId`] variant category.
pub(super) fn output_type_label(output: &OutputId) -> &'static str {
    match output {
        OutputId::Axis { .. } => "Axis",
        OutputId::Button { .. } => "Button",
        OutputId::Hat { .. } => "Hat",
    }
}

/// Map-to-vJoy configuration: device, output type, and axis/button/hat selector.
pub(super) fn show_map_to_vjoy(
    ui: &mut egui::Ui,
    output: &mut inputforge_core::types::OutputAddress,
    colors: &ThemeColors,
    virtual_devices: &[VirtualDeviceConfig],
) -> bool {
    description(
        ui,
        "Routes this input to a virtual vJoy device output.",
        colors,
    );

    let mut changed = false;

    egui::Grid::new(ui.id().with("vjoy_config"))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            changed |= vjoy_device_selector(ui, output, colors, virtual_devices);
            changed |= vjoy_output_type_selector(ui, output, colors);
            changed |= vjoy_output_value_selector(ui, output, colors, virtual_devices);
        });

    changed
}

/// Device selector row — uses discovered devices when available,
/// falls back to 1..=16 until the engine populates the list.
fn vjoy_device_selector(
    ui: &mut egui::Ui,
    output: &mut inputforge_core::types::OutputAddress,
    colors: &ThemeColors,
    virtual_devices: &[VirtualDeviceConfig],
) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("Device").color(colors.text_dim));
    let current_device_label = format!("Device {}", output.device);
    egui::ComboBox::from_id_salt(ui.id().with("vjoy_device"))
        .selected_text(current_device_label)
        .width(150.0)
        .show_ui(ui, |ui| {
            let device_ids: Vec<u8> = if virtual_devices.is_empty() {
                (1..=16_u8).collect()
            } else {
                virtual_devices.iter().map(|d| d.device_id).collect()
            };
            for device_id in device_ids {
                let label = format!("Device {device_id}");
                if ui
                    .selectable_value(&mut output.device, device_id, label)
                    .changed()
                {
                    changed = true;
                }
            }
        });
    ui.end_row();
    changed
}

/// Output type selector row (Axis / Button / Hat).
fn vjoy_output_type_selector(
    ui: &mut egui::Ui,
    output: &mut inputforge_core::types::OutputAddress,
    colors: &ThemeColors,
) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("Type").color(colors.text_dim));
    let current_type = output_type_label(&output.output);
    egui::ComboBox::from_id_salt(ui.id().with("vjoy_output_type"))
        .selected_text(current_type)
        .width(150.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(matches!(output.output, OutputId::Axis { .. }), "Axis")
                .clicked()
                && !matches!(output.output, OutputId::Axis { .. })
            {
                output.output = OutputId::Axis { id: VJoyAxis::X };
                changed = true;
            }
            if ui
                .selectable_label(matches!(output.output, OutputId::Button { .. }), "Button")
                .clicked()
                && !matches!(output.output, OutputId::Button { .. })
            {
                output.output = OutputId::Button { id: 1 };
                changed = true;
            }
            if ui
                .selectable_label(matches!(output.output, OutputId::Hat { .. }), "Hat")
                .clicked()
                && !matches!(output.output, OutputId::Hat { .. })
            {
                output.output = OutputId::Hat { id: 1 };
                changed = true;
            }
        });
    ui.end_row();
    changed
}

/// Variant-specific value selector row (axis picker, button combo, hat combo).
fn vjoy_output_value_selector(
    ui: &mut egui::Ui,
    output: &mut inputforge_core::types::OutputAddress,
    colors: &ThemeColors,
    virtual_devices: &[VirtualDeviceConfig],
) -> bool {
    let mut changed = false;
    match &mut output.output {
        OutputId::Axis { id } => {
            ui.label(egui::RichText::new("Axis").color(colors.text_dim));
            let current_axis_label = vjoy_axis_label(*id);
            egui::ComboBox::from_id_salt(ui.id().with("vjoy_axis"))
                .selected_text(current_axis_label)
                .width(150.0)
                .show_ui(ui, |ui| {
                    for axis in &VJOY_AXES {
                        if ui
                            .selectable_value(id, *axis, vjoy_axis_label(*axis))
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
            ui.end_row();
        }
        OutputId::Button { id } => {
            ui.label(egui::RichText::new("Button").color(colors.text_dim));
            let max_buttons = virtual_devices
                .iter()
                .find(|d| d.device_id == output.device)
                .map_or(128_u8, |d| d.button_count);
            egui::ComboBox::from_id_salt(ui.id().with("vjoy_button"))
                .selected_text(format!("Button {id}"))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for btn in 1..=max_buttons {
                        if ui
                            .selectable_value(id, btn, format!("Button {btn}"))
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
            ui.end_row();
        }
        OutputId::Hat { id } => {
            ui.label(egui::RichText::new("Hat").color(colors.text_dim));
            let max_hats = virtual_devices
                .iter()
                .find(|d| d.device_id == output.device)
                .map_or(4_u8, |d| d.hat_count);
            egui::ComboBox::from_id_salt(ui.id().with("vjoy_hat"))
                .selected_text(format!("Hat {id}"))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for hat in 1..=max_hats {
                        if ui.selectable_value(id, hat, format!("Hat {hat}")).changed() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vjoy_axis_labels_are_nonempty() {
        for axis in VJOY_AXES {
            assert!(!vjoy_axis_label(axis).is_empty());
        }
    }

    #[test]
    fn output_type_label_covers_all_variants() {
        assert_eq!(
            output_type_label(&OutputId::Axis { id: VJoyAxis::X }),
            "Axis"
        );
        assert_eq!(output_type_label(&OutputId::Button { id: 1 }), "Button");
        assert_eq!(output_type_label(&OutputId::Hat { id: 1 }), "Hat");
    }
}
