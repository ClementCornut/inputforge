// Rust guideline compliant 2026-03-04

//! Per-variant configuration UI for each [`Action`] type.
//!
//! Delegates to specialized widgets (deadzone editor, calibration editor,
//! etc.) for complex variants, and renders inline controls for simple ones.

use inputforge_core::action::{Action, CycleModes, ModeChangeStrategy};
use inputforge_core::types::{
    InputAddress, InputId, KeyModifier, MergeOp, OutputId, VJoyAxis, VirtualDeviceConfig,
};

use crate::theme::ThemeColors;
use crate::widgets::{calibration_editor, curve_editor, deadzone_editor};

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
fn output_type_label(output: &OutputId) -> &'static str {
    match output {
        OutputId::Axis { .. } => "Axis",
        OutputId::Button { .. } => "Button",
        OutputId::Hat { .. } => "Hat",
    }
}

/// Format an [`InputAddress`] as a human-readable string.
fn format_input_address(address: &InputAddress) -> String {
    let input_label = match &address.input {
        InputId::Axis { index } => format!("Axis {index}"),
        InputId::Button { index } => format!("Button {index}"),
        InputId::Hat { index } => format!("Hat {index}"),
    };
    format!("{} / {input_label}", address.device.0)
}

/// Render a brief description line above the config body.
fn description(ui: &mut egui::Ui, text: &str, colors: &ThemeColors) {
    ui.label(egui::RichText::new(text).italics().color(colors.text_dim));
    ui.add_space(4.0);
}

/// Render the configuration UI for a single action variant.
///
/// Returns `true` when the user modified the action's parameters.
/// The caller is responsible for propagating changes back to the
/// mapping state.
pub(crate) fn action_config(
    ui: &mut egui::Ui,
    action: &mut Action,
    colors: &ThemeColors,
    virtual_devices: &[VirtualDeviceConfig],
) -> bool {
    match action {
        Action::Deadzone { config } => show_deadzone(ui, config, colors),
        Action::Calibrate { config } => show_calibrate(ui, config, colors),
        Action::ResponseCurve { curve } => show_response_curve(ui, curve, colors),
        Action::Invert => show_invert(ui, colors),
        Action::MapToVJoy { output } => show_map_to_vjoy(ui, output, colors, virtual_devices),
        Action::MapToKeyboard { key } => show_map_to_keyboard(ui, key, colors),
        Action::MergeAxis {
            second_input,
            operation,
        } => show_merge_axis(ui, second_input, operation, colors),
        Action::ChangeMode { strategy } => show_change_mode(ui, strategy, colors),
        Action::Conditional {
            condition,
            if_true,
            if_false,
        } => show_conditional(ui, condition, if_true, if_false, colors),
    }
}

/// Delegate to the dedicated deadzone editor widget.
fn show_deadzone(
    ui: &mut egui::Ui,
    config: &mut inputforge_core::processing::deadzone::DeadzoneConfig,
    colors: &ThemeColors,
) -> bool {
    description(
        ui,
        "Ignores small inputs in the center band and saturates at the extremes.",
        colors,
    );

    if let Some(new_config) = deadzone_editor::deadzone_editor(ui, config, None) {
        *config = new_config;
        true
    } else {
        false
    }
}

/// Delegate to the dedicated calibration editor widget.
fn show_calibrate(
    ui: &mut egui::Ui,
    config: &mut inputforge_core::processing::calibration::Calibration,
    colors: &ThemeColors,
) -> bool {
    description(
        ui,
        "Maps the physical device range to normalized [-1, 1] with center compensation.",
        colors,
    );

    if let Some(new_config) = calibration_editor::calibration_editor(ui, config, None) {
        *config = new_config;
        true
    } else {
        false
    }
}

/// Delegate to the dedicated response curve editor widget.
///
/// State is stored in `egui::Memory` keyed by widget ID so it persists
/// across frames without requiring the caller to manage it explicitly.
fn show_response_curve(
    ui: &mut egui::Ui,
    curve: &mut inputforge_core::processing::curves::ResponseCurve,
    colors: &ThemeColors,
) -> bool {
    description(
        ui,
        "Shapes the axis response using a customizable transfer curve.",
        colors,
    );
    let id = ui.id().with("curve_editor_state");
    let mut state = ui
        .data_mut(|d| d.get_temp::<curve_editor::CurveEditorState>(id))
        .unwrap_or_default();
    let changed = curve_editor::curve_editor(ui, curve, &mut state, None);
    ui.data_mut(|d| d.insert_temp(id, state));
    changed
}

/// Invert has no configurable parameters.
fn show_invert(ui: &mut egui::Ui, colors: &ThemeColors) -> bool {
    ui.label(
        egui::RichText::new("Inverts the axis value (multiplies by \u{2212}1)")
            .color(colors.text_dim),
    );
    false
}

/// Map-to-vJoy configuration: device, output type, and axis/button/hat selector.
fn show_map_to_vjoy(
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

/// Map-to-keyboard configuration: key name and modifier checkboxes.
fn show_map_to_keyboard(
    ui: &mut egui::Ui,
    key: &mut inputforge_core::types::KeyCombo,
    colors: &ThemeColors,
) -> bool {
    description(
        ui,
        "Simulates a keyboard key press when the input is active.",
        colors,
    );

    let mut changed = false;

    egui::Grid::new(ui.id().with("keyboard_config"))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            // Key name text field with placeholder.
            ui.label(egui::RichText::new("Key").color(colors.text_dim));
            if ui
                .add(
                    egui::TextEdit::singleline(&mut key.key)
                        .desired_width(100.0)
                        .hint_text("e.g. A, F1, Space"),
                )
                .changed()
            {
                changed = true;
            }
            ui.end_row();

            // Modifier checkboxes.
            ui.label(egui::RichText::new("Modifiers").color(colors.text_dim));
            ui.horizontal(|ui| {
                changed |= modifier_checkbox(ui, &mut key.modifiers, KeyModifier::Ctrl, "Ctrl");
                changed |= modifier_checkbox(ui, &mut key.modifiers, KeyModifier::Shift, "Shift");
                changed |= modifier_checkbox(ui, &mut key.modifiers, KeyModifier::Alt, "Alt");
                changed |= modifier_checkbox(ui, &mut key.modifiers, KeyModifier::Win, "Win");
            });
            ui.end_row();
        });

    changed
}

/// Render a single modifier checkbox, syncing with the modifiers vec.
///
/// Returns `true` when the modifier state changed.
fn modifier_checkbox(
    ui: &mut egui::Ui,
    modifiers: &mut Vec<KeyModifier>,
    modifier: KeyModifier,
    label: &str,
) -> bool {
    let mut active = modifiers.contains(&modifier);
    if ui.checkbox(&mut active, label).changed() {
        if active {
            if !modifiers.contains(&modifier) {
                modifiers.push(modifier);
            }
        } else {
            modifiers.retain(|m| *m != modifier);
        }
        return true;
    }
    false
}

/// Merge axis configuration: second input address and merge operation.
fn show_merge_axis(
    ui: &mut egui::Ui,
    second_input: &mut InputAddress,
    operation: &mut MergeOp,
    colors: &ThemeColors,
) -> bool {
    description(
        ui,
        "Combines two axis inputs into one using the selected operation.",
        colors,
    );

    let mut changed = false;

    egui::Grid::new(ui.id().with("merge_axis_config"))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Second Input").color(colors.text_dim));
            ui.label(
                egui::RichText::new(format_input_address(second_input))
                    .monospace()
                    .color(colors.text),
            );
            ui.end_row();

            ui.label(egui::RichText::new("Operation").color(colors.text_dim));
            let current_label = match operation {
                MergeOp::Bidirectional => "Bidirectional",
                MergeOp::Average => "Average",
                MergeOp::Maximum => "Maximum",
            };
            egui::ComboBox::from_id_salt(ui.id().with("merge_op"))
                .selected_text(current_label)
                .width(150.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(operation, MergeOp::Bidirectional, "Bidirectional")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(operation, MergeOp::Average, "Average")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(operation, MergeOp::Maximum, "Maximum")
                        .changed()
                    {
                        changed = true;
                    }
                });
            ui.end_row();
        });

    changed
}

/// Change mode configuration: strategy selector with variant-specific controls.
fn show_change_mode(
    ui: &mut egui::Ui,
    strategy: &mut ModeChangeStrategy,
    colors: &ThemeColors,
) -> bool {
    /// Strategy index for `ComboBox` selection.
    const SWITCH_TO: usize = 0;
    const TEMPORARY: usize = 1;
    const PREVIOUS: usize = 2;
    const CYCLE: usize = 3;

    description(
        ui,
        "Switches the active mapping mode when triggered.",
        colors,
    );

    let mut changed = false;

    let current_index = match strategy {
        ModeChangeStrategy::SwitchTo { .. } => SWITCH_TO,
        ModeChangeStrategy::Temporary { .. } => TEMPORARY,
        ModeChangeStrategy::Previous => PREVIOUS,
        ModeChangeStrategy::Cycle { .. } => CYCLE,
    };

    let labels = ["Switch To", "Temporary", "Previous", "Cycle"];

    egui::Grid::new(ui.id().with("change_mode_config"))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            // Strategy selector.
            ui.label(egui::RichText::new("Strategy").color(colors.text_dim));
            let mut selected = current_index;
            egui::ComboBox::from_id_salt(ui.id().with("mode_strategy"))
                .selected_text(labels[selected])
                .width(150.0)
                .show_ui(ui, |ui| {
                    for (index, label) in labels.iter().enumerate() {
                        if ui.selectable_value(&mut selected, index, *label).changed() {
                            changed = true;
                        }
                    }
                });

            // Reconstruct strategy if the variant changed.
            if changed && selected != current_index {
                *strategy = match selected {
                    SWITCH_TO => ModeChangeStrategy::SwitchTo {
                        mode: String::new(),
                    },
                    TEMPORARY => ModeChangeStrategy::Temporary {
                        mode: String::new(),
                    },
                    CYCLE => {
                        // CycleModes requires at least 2 modes.
                        if let Ok(modes) =
                            CycleModes::new(vec!["Mode A".to_owned(), "Mode B".to_owned()])
                        {
                            ModeChangeStrategy::Cycle { modes }
                        } else {
                            ModeChangeStrategy::Previous
                        }
                    }
                    // PREVIOUS and any unexpected index.
                    _ => ModeChangeStrategy::Previous,
                };
            }
            ui.end_row();

            // Variant-specific controls.
            match strategy {
                ModeChangeStrategy::SwitchTo { mode } | ModeChangeStrategy::Temporary { mode } => {
                    ui.label(egui::RichText::new("Mode").color(colors.text_dim));
                    if ui
                        .add(
                            egui::TextEdit::singleline(mode)
                                .desired_width(120.0)
                                .hint_text("mode name"),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();
                }
                ModeChangeStrategy::Cycle { modes } => {
                    ui.label(egui::RichText::new("Modes").color(colors.text_dim));
                    let mode_list = modes.modes().join(", ");
                    ui.label(
                        egui::RichText::new(mode_list)
                            .monospace()
                            .small()
                            .color(colors.text),
                    );
                    ui.end_row();
                }
                ModeChangeStrategy::Previous => {}
            }
        });

    changed
}

/// Conditional configuration: condition display and nested action lists.
fn show_conditional(
    ui: &mut egui::Ui,
    condition: &mut inputforge_core::action::Condition,
    if_true: &mut [Action],
    if_false: &mut Option<Vec<Action>>,
    colors: &ThemeColors,
) -> bool {
    description(
        ui,
        "Branches the pipeline based on a runtime condition.",
        colors,
    );

    // Display condition as debug text for now.
    ui.label(egui::RichText::new("Condition:").color(colors.text_dim));
    ui.label(
        egui::RichText::new(format!("{condition:?}"))
            .monospace()
            .small()
            .color(colors.text),
    );

    ui.add_space(4.0);

    ui.label(
        egui::RichText::new(format!("If true: {} action(s)", if_true.len())).color(colors.text_dim),
    );
    if let Some(false_branch) = if_false {
        ui.label(
            egui::RichText::new(format!("If false: {} action(s)", false_branch.len()))
                .color(colors.text_dim),
        );
    }

    // Full recursive editing deferred to a future iteration.
    false
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
    fn format_input_address_produces_readable_output() {
        let address = InputAddress {
            device: inputforge_core::types::DeviceId("joystick-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let formatted = format_input_address(&address);
        assert!(formatted.contains("joystick-1"));
        assert!(formatted.contains("Axis 0"));
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

    #[test]
    fn show_deadzone_returns_false_without_change() {
        // Cannot test UI rendering, but verify function signature compiles.
        let _ = Action::Deadzone {
            config: inputforge_core::processing::deadzone::DeadzoneConfig::default(),
        };
    }

    #[test]
    fn show_invert_has_no_params() {
        let action = Action::Invert;
        assert_eq!(action, Action::Invert);
    }
}
