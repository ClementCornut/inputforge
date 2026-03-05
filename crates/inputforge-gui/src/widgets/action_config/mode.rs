// Rust guideline compliant 2026-03-04

//! Mode change configuration UI.
//!
//! Renders the strategy selector and variant-specific controls for the
//! [`Action::ChangeMode`] variant.

use inputforge_core::action::{CycleModes, ModeChangeStrategy};

use crate::theme::{SMALL_FONT_SIZE, ThemeColors};

use super::description;

/// Change mode configuration: strategy selector with variant-specific controls.
pub(super) fn show_change_mode(
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
                            .size(SMALL_FONT_SIZE)
                            .color(colors.text),
                    );
                    ui.end_row();
                }
                ModeChangeStrategy::Previous => {}
            }
        });

    changed
}
