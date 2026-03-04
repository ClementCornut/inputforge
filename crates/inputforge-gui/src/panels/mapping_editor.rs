// Rust guideline compliant 2026-03-04

//! Mapping editor central panel.
//!
//! Displays an editable pipeline of [`Action`] cards for the currently
//! selected input. Actions can be reordered via up/down arrow buttons,
//! added from a categorized dropdown, or deleted individually.

use std::collections::HashSet;

use egui::{CornerRadius, Margin, Pos2, Rect, Stroke};

use inputforge_core::action::Action;

use crate::app::{CachedState, GuiSelection};
use crate::theme;
use crate::widgets::{action_card, action_config, empty_state};

/// Persistent state for the mapping editor panel.
#[derive(Debug, Default)]
pub(crate) struct MappingEditorState {
    /// Working copy of the action pipeline being edited.
    actions: Vec<Action>,
    /// Indices of expanded action cards.
    expanded: HashSet<usize>,
    /// Whether the working copy has unsaved changes.
    dirty: bool,
}

impl MappingEditorState {
    /// Create a new empty mapping editor state.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

/// Render the mapping editor panel.
///
/// If no device or input is selected, shows a placeholder message.
/// Otherwise displays the action pipeline with arrow-button
/// reordering, add/delete controls, and per-action configuration.
pub(crate) fn show(
    ui: &mut egui::Ui,
    state: &mut MappingEditorState,
    cache: &CachedState,
    _selection: &GuiSelection,
) {
    let colors = theme::colors(ui.ctx());

    // Header.
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Action Pipeline")
                .family(egui::FontFamily::Name("SemiBold".into()))
                .color(colors.text),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if state.dirty {
                ui.label(egui::RichText::new("unsaved").small().color(colors.warning));
            }
        });
    });

    ui.separator();

    // Scrollable action card list.
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            show_action_list(ui, state, cache, colors);

            ui.add_space(8.0);

            // "Add Action" dropdown at the bottom.
            show_add_action_dropdown(ui, state, colors);
        });
}

/// Render the list of action cards with arrow-button reordering.
fn show_action_list(
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
        reindex_expanded_after_swap(&mut state.expanded, idx, idx - 1);
        state.dirty = true;
    }

    // Handle move down (swap with next).
    if let Some(idx) = move_down_index {
        state.actions.swap(idx, idx + 1);
        reindex_expanded_after_swap(&mut state.expanded, idx, idx + 1);
        state.dirty = true;
    }

    // Handle deletion.
    if let Some(idx) = delete_index {
        state.actions.remove(idx);
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

/// Update the expanded set after swapping two adjacent actions.
fn reindex_expanded_after_swap(expanded: &mut HashSet<usize>, index_a: usize, index_b: usize) {
    let a_was_expanded = expanded.remove(&index_a);
    let b_was_expanded = expanded.remove(&index_b);
    if a_was_expanded {
        expanded.insert(index_b);
    }
    if b_was_expanded {
        expanded.insert(index_a);
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

            // Expanded config body — inside the card frame.
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
fn show_add_action_dropdown(
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
                        .small()
                        .color(colors.processing()),
                );
                if ui.button("Response Curve").clicked() {
                    add_default_response_curve(state);
                    ui.close();
                }
                if ui.button("Deadzone").clicked() {
                    state.actions.push(Action::Deadzone {
                        config: inputforge_core::processing::deadzone::DeadzoneConfig::default(),
                    });
                    state.dirty = true;
                    ui.close();
                }
                if ui.button("Calibrate").clicked() {
                    // Use validated constructor with sensible defaults.
                    if let Ok(config) = inputforge_core::processing::calibration::Calibration::new(
                        -1.0, -0.05, 0.05, 1.0, true,
                    ) {
                        state.actions.push(Action::Calibrate { config });
                        state.dirty = true;
                    }
                    ui.close();
                }
                if ui.button("Invert").clicked() {
                    state.actions.push(Action::Invert);
                    state.dirty = true;
                    ui.close();
                }

                ui.separator();

                // Output section.
                ui.label(
                    egui::RichText::new("OUTPUT")
                        .monospace()
                        .small()
                        .color(colors.output()),
                );
                if ui.button("Map to vJoy").clicked() {
                    state.actions.push(Action::MapToVJoy {
                        output: inputforge_core::types::OutputAddress {
                            device: 1,
                            output: inputforge_core::types::OutputId::Axis {
                                id: inputforge_core::types::VJoyAxis::X,
                            },
                        },
                    });
                    state.dirty = true;
                    ui.close();
                }
                if ui.button("Map to Keyboard").clicked() {
                    state.actions.push(Action::MapToKeyboard {
                        key: inputforge_core::types::KeyCombo {
                            key: String::new(),
                            modifiers: vec![],
                        },
                    });
                    state.dirty = true;
                    ui.close();
                }
                if ui.button("Merge Axis").clicked() {
                    state.actions.push(Action::MergeAxis {
                        second_input: inputforge_core::types::InputAddress {
                            device: inputforge_core::types::DeviceId(String::new()),
                            input: inputforge_core::types::InputId::Axis { index: 0 },
                        },
                        operation: inputforge_core::types::MergeOp::Bidirectional,
                    });
                    state.dirty = true;
                    ui.close();
                }

                ui.separator();

                // Control section.
                ui.label(
                    egui::RichText::new("CONTROL")
                        .monospace()
                        .small()
                        .color(colors.control()),
                );
                if ui.button("Change Mode").clicked() {
                    state.actions.push(Action::ChangeMode {
                        strategy: inputforge_core::action::ModeChangeStrategy::Previous,
                    });
                    state.dirty = true;
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
        state.actions.push(Action::ResponseCurve { curve });
        state.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty_and_clean() {
        let state = MappingEditorState::new();
        assert!(state.actions.is_empty());
        assert!(state.expanded.is_empty());
        assert!(!state.dirty);
    }

    #[test]
    fn add_default_response_curve_adds_identity() {
        let mut state = MappingEditorState::new();
        add_default_response_curve(&mut state);
        assert_eq!(state.actions.len(), 1);
        assert!(state.dirty);
        assert!(matches!(&state.actions[0], Action::ResponseCurve { .. }));
    }

    #[test]
    fn reindex_expanded_after_swap_tracks_both() {
        let mut expanded = HashSet::from([0, 2]);
        reindex_expanded_after_swap(&mut expanded, 0, 1);
        // 0 was expanded -> moves to 1; 1 was not -> stays not at 0.
        assert!(!expanded.contains(&0));
        assert!(expanded.contains(&1));
        assert!(expanded.contains(&2));
    }

    #[test]
    fn reindex_expanded_after_swap_both_expanded() {
        let mut expanded = HashSet::from([1, 2]);
        reindex_expanded_after_swap(&mut expanded, 1, 2);
        // Both swapped — both remain expanded at swapped positions.
        assert!(expanded.contains(&1));
        assert!(expanded.contains(&2));
    }
}
