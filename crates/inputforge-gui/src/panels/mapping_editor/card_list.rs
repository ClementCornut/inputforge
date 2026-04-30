// Rust guideline compliant 2026-03-04

//! Action card list and dropdown for the mapping editor.
//!
//! Contains the scrollable list of action cards with reordering support
//! and the categorized "Add Action" dropdown menu.

use std::collections::HashSet;

use egui::{CornerRadius, Margin, Pos2, Rect, Stroke};

use inputforge_core::action::Action;

use super::{MappingEditorState, reindex_expanded_after_swap};
use crate::app::CachedState;
use crate::theme;
use crate::widgets::{action_card, action_config, empty_state};

/// Render the list of action cards with arrow-button reordering.
pub(super) fn show_action_list(
    ui: &mut egui::Ui,
    state: &mut MappingEditorState,
    cache: &CachedState,
    colors: &theme::ThemeColors,
) {
    if state.actions.is_empty() {
        empty_state::empty_state(ui, "No actions yet \u{2014} add one below");
        return;
    }

    let mut delete_index: Option<usize> = None;
    let mut toggle_index: Option<usize> = None;
    let mut move_up_index: Option<usize> = None;
    let mut move_down_index: Option<usize> = None;
    let count = state.actions.len();

    debug_assert_eq!(
        state.actions.len(),
        state.action_ids.len(),
        "actions and action_ids must be kept in sync"
    );

    for index in 0..count {
        if index > 0 {
            action_card::flow_connector(ui, colors);
        }

        let expanded = state.expanded.contains(&index);
        let can_move_up = index > 0;
        let can_move_down = index + 1 < count;

        render_single_card(
            ui,
            state,
            cache,
            index,
            expanded,
            can_move_up,
            can_move_down,
            colors,
            &mut delete_index,
            &mut toggle_index,
            &mut move_up_index,
            &mut move_down_index,
        );
    }

    // Handle expand/collapse toggling.
    if let Some(idx) = toggle_index {
        if state.expanded.contains(&idx) {
            state.expanded.remove(&idx);
        } else {
            state.expanded.insert(idx);
        }
    }

    // Handle move up (swap with previous).
    if let Some(idx) = move_up_index {
        state.actions.swap(idx, idx - 1);
        state.action_ids.swap(idx, idx - 1);
        reindex_expanded_after_swap(&mut state.expanded, idx, idx - 1);
        state.dirty = true;
    }

    // Handle move down (swap with next).
    if let Some(idx) = move_down_index {
        state.actions.swap(idx, idx + 1);
        state.action_ids.swap(idx, idx + 1);
        reindex_expanded_after_swap(&mut state.expanded, idx, idx + 1);
        state.dirty = true;
    }

    // Handle deletion.
    if let Some(idx) = delete_index {
        state.actions.remove(idx);
        state.action_ids.remove(idx);
        state.expanded.remove(&idx);
        let new_expanded: HashSet<usize> = state
            .expanded
            .iter()
            .map(|&expanded_idx| {
                if expanded_idx > idx {
                    expanded_idx - 1
                } else {
                    expanded_idx
                }
            })
            .collect();
        state.expanded = new_expanded;
        state.dirty = true;
    }
}

/// Render a single action card wrapped in a unified frame with accent bar,
/// header, and optional expanded config body.
#[expect(
    clippy::too_many_arguments,
    reason = "collecting multiple response outputs"
)]
fn render_single_card(
    ui: &mut egui::Ui,
    state: &mut MappingEditorState,
    cache: &CachedState,
    index: usize,
    expanded: bool,
    can_move_up: bool,
    can_move_down: bool,
    colors: &theme::ThemeColors,
    delete_index: &mut Option<usize>,
    toggle_index: &mut Option<usize>,
    move_up_index: &mut Option<usize>,
    move_down_index: &mut Option<usize>,
) {
    let accent_color = action_card::category_accent_color(&state.actions[index], colors);

    // Left margin reserves space for the accent bar painted as an overlay.
    let accent_width = action_card::ACCENT_BAR_WIDTH;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "accent_width + gap is always small enough for i8"
    )]
    let left_margin = (accent_width as i8) + 4;

    let frame_response = egui::Frame::new()
        .fill(colors.mantle)
        .stroke(Stroke::new(1.0, colors.surface1))
        .corner_radius(6.0)
        .inner_margin(Margin {
            left: left_margin,
            right: 8,
            top: 4,
            bottom: 4,
        })
        .show(ui, |ui| {
            // Push stable action ID so all child widgets get unique IDs.
            let action_id = state.action_ids[index];
            ui.push_id(action_id, |ui| {
                // Header row with action name, category badge, and buttons.
                let card_response = action_card::action_card(
                    ui,
                    &state.actions[index],
                    expanded,
                    can_move_up,
                    can_move_down,
                    colors,
                );

                match card_response {
                    action_card::ActionCardResponse::Delete => *delete_index = Some(index),
                    action_card::ActionCardResponse::Toggle => *toggle_index = Some(index),
                    action_card::ActionCardResponse::MoveUp => *move_up_index = Some(index),
                    action_card::ActionCardResponse::MoveDown => {
                        *move_down_index = Some(index);
                    }
                    action_card::ActionCardResponse::None => {}
                }

                // Expanded config body, inside the card frame.
                if expanded {
                    ui.separator();
                    ui.add_space(4.0);

                    egui::Frame::new().inner_margin(8.0).show(ui, |ui| {
                        if action_config::action_config(
                            ui,
                            &mut state.actions[index],
                            colors,
                            &cache.virtual_devices,
                        ) {
                            state.dirty = true;
                        }
                    });
                }
            });
        });

    // Paint accent bar as overlay using the frame's actual rect.
    // This ensures it spans the full card height regardless of content.
    let card_rect = frame_response.response.rect;
    let accent_rect = Rect::from_min_max(
        card_rect.left_top(),
        Pos2::new(card_rect.left() + accent_width, card_rect.bottom()),
    );
    let rounding = CornerRadius {
        nw: 6,
        sw: 6,
        ne: 0,
        se: 0,
    };
    ui.painter()
        .rect_filled(accent_rect, rounding, accent_color);
}

/// Render the "Add Action" dropdown with categorized sections.
pub(super) fn show_add_action_dropdown(
    ui: &mut egui::Ui,
    state: &mut MappingEditorState,
    colors: &theme::ThemeColors,
) {
    ui.horizontal(|ui| {
        ui.menu_button(
            egui::RichText::new("+ Add Action").color(colors.primary),
            |ui| {
                // Processing section.
                ui.label(
                    egui::RichText::new("PROCESSING")
                        .monospace()
                        .size(theme::SMALL_FONT_SIZE)
                        .color(colors.processing()),
                );
                if ui.button("Response Curve").clicked() {
                    add_default_response_curve(state);
                    ui.close();
                }
                if ui.button("Deadzone").clicked() {
                    state.push_action(Action::Deadzone {
                        config: inputforge_core::processing::deadzone::DeadzoneConfig::default(),
                    });
                    ui.close();
                }
                if ui.button("Invert").clicked() {
                    state.push_action(Action::Invert);
                    ui.close();
                }

                ui.separator();

                // Output section.
                ui.label(
                    egui::RichText::new("OUTPUT")
                        .monospace()
                        .size(theme::SMALL_FONT_SIZE)
                        .color(colors.output()),
                );
                if ui.button("Map to vJoy").clicked() {
                    state.push_action(Action::MapToVJoy {
                        output: inputforge_core::types::OutputAddress {
                            device: 1,
                            output: inputforge_core::types::OutputId::Axis {
                                id: inputforge_core::types::VJoyAxis::X,
                            },
                        },
                    });
                    ui.close();
                }
                if ui.button("Map to Keyboard").clicked() {
                    state.push_action(Action::MapToKeyboard {
                        key: inputforge_core::types::KeyCombo {
                            key: String::new(),
                            modifiers: vec![],
                        },
                    });
                    ui.close();
                }
                if ui.button("Merge Axis").clicked() {
                    state.push_action(Action::MergeAxis {
                        second_input: inputforge_core::types::InputAddress {
                            device: inputforge_core::types::DeviceId(String::new()),
                            input: inputforge_core::types::InputId::Axis { index: 0 },
                        },
                        operation: inputforge_core::types::MergeOp::Bidirectional,
                    });
                    ui.close();
                }

                ui.separator();

                // Control section.
                ui.label(
                    egui::RichText::new("CONTROL")
                        .monospace()
                        .size(theme::SMALL_FONT_SIZE)
                        .color(colors.control()),
                );
                if ui.button("Change Mode").clicked() {
                    state.push_action(Action::ChangeMode {
                        strategy: inputforge_core::action::ModeChangeStrategy::Previous,
                    });
                    ui.close();
                }
            },
        );
    });
}

/// Add a default identity response curve to the pipeline.
fn add_default_response_curve(state: &mut MappingEditorState) {
    // Identity curve: straight line from (-1, -1) to (1, 1).
    let curve = inputforge_core::processing::ResponseCurve::piecewise_linear(
        vec![(-1.0, -1.0), (1.0, 1.0)],
        false,
    );
    if let Ok(curve) = curve {
        state.push_action(Action::ResponseCurve { curve });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_default_response_curve_adds_identity() {
        let mut state = MappingEditorState::new();
        add_default_response_curve(&mut state);
        assert_eq!(state.actions.len(), 1);
        assert!(state.dirty);
        assert!(matches!(&state.actions[0], Action::ResponseCurve { .. }));
    }
}
