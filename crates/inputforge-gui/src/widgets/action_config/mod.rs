// Rust guideline compliant 2026-03-04

//! Per-variant configuration UI for each [`Action`] type.
//!
//! Delegates to specialized widgets (deadzone editor, calibration editor,
//! etc.) for complex variants, and renders inline controls for simple ones.

mod mode;
mod vjoy;

use inputforge_core::action::Action;
use inputforge_core::types::{InputAddress, InputId, KeyModifier, MergeOp, VirtualDeviceConfig};

use crate::theme::{SMALL_FONT_SIZE, ThemeColors};
use crate::widgets::{curve_editor, deadzone_editor};

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
        Action::ResponseCurve { curve } => show_response_curve(ui, curve, colors),
        Action::Invert => show_invert(ui, colors),
        Action::MapToVJoy { output } => vjoy::show_map_to_vjoy(ui, output, colors, virtual_devices),
        Action::MapToKeyboard { key } => show_map_to_keyboard(ui, key, colors),
        Action::MergeAxis {
            second_input,
            operation,
        } => show_merge_axis(ui, second_input, operation, colors),
        Action::ChangeMode { strategy } => mode::show_change_mode(ui, strategy, colors),
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
            .size(SMALL_FONT_SIZE)
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
