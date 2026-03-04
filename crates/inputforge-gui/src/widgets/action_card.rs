// Rust guideline compliant 2026-03-03

//! Reusable action card widget for the mapping editor pipeline.
//!
//! Provides the header row (action name, category badge, control buttons)
//! and helpers (accent bar, flow connectors) consumed by the mapping
//! editor panel, which owns the outer card frame and expanded body.

use egui::{Color32, CursorIcon, FontFamily, Pos2, Stroke, Vec2};

use inputforge_core::action::Action;

use crate::theme::ThemeColors;

/// Return the accent color for an [`Action`] based on its pipeline category.
pub(crate) fn category_accent_color(action: &Action, colors: &ThemeColors) -> Color32 {
    ActionCategory::of(action).color(colors)
}

/// Width of the left accent bar in logical pixels.
pub(crate) const ACCENT_BAR_WIDTH: f32 = 4.0;

/// Uniform minimum size for card control buttons (consistent hover rects).
const BUTTON_MIN_SIZE: Vec2 = Vec2::new(20.0, 20.0);

/// Height of the vertical flow connector between cards.
const FLOW_CONNECTOR_HEIGHT: f32 = 8.0;

/// Width of the flow connector line.
const FLOW_CONNECTOR_WIDTH: f32 = 2.0;

/// Response from rendering an action card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionCardResponse {
    /// No interaction occurred.
    None,
    /// User clicked the delete button on this card.
    Delete,
    /// User clicked the header to toggle expand/collapse.
    Toggle,
    /// User clicked the move-up arrow.
    MoveUp,
    /// User clicked the move-down arrow.
    MoveDown,
}

/// Action pipeline category, used for color coding and labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionCategory {
    Processing,
    Output,
    Control,
}

impl ActionCategory {
    /// Classify an [`Action`] into its pipeline category.
    const fn of(action: &Action) -> Self {
        match action {
            Action::ResponseCurve { .. }
            | Action::Deadzone { .. }
            | Action::Calibrate { .. }
            | Action::Invert => Self::Processing,

            Action::MapToVJoy { .. } | Action::MapToKeyboard { .. } | Action::MergeAxis { .. } => {
                Self::Output
            }

            Action::ChangeMode { .. } | Action::Conditional { .. } => Self::Control,
        }
    }

    /// Accent color for this category from the active theme.
    fn color(self, colors: &ThemeColors) -> Color32 {
        match self {
            Self::Processing => colors.processing(),
            Self::Output => colors.output(),
            Self::Control => colors.control(),
        }
    }

    /// Short uppercase label for the category badge.
    const fn label(self) -> &'static str {
        match self {
            Self::Processing => "PROCESSING",
            Self::Output => "OUTPUT",
            Self::Control => "CONTROL",
        }
    }
}

/// Human-readable display name for an action variant.
fn action_name(action: &Action) -> &'static str {
    match action {
        Action::ResponseCurve { .. } => "Response Curve",
        Action::Deadzone { .. } => "Deadzone",
        Action::Calibrate { .. } => "Calibrate",
        Action::Invert => "Invert",
        Action::MapToVJoy { .. } => "Map to vJoy",
        Action::MapToKeyboard { .. } => "Map to Keyboard",
        Action::MergeAxis { .. } => "Merge Axis",
        Action::ChangeMode { .. } => "Change Mode",
        Action::Conditional { .. } => "Conditional",
    }
}

/// Render the action card header row (name, category badge, control buttons).
///
/// Returns an [`ActionCardResponse`] indicating whether the user
/// interacted with the card (e.g., clicked delete or a move arrow).
///
/// The caller is responsible for wrapping this in a [`egui::Frame`]
/// with accent bar; see `mapping_editor::render_single_card`.
pub(crate) fn action_card(
    ui: &mut egui::Ui,
    action: &Action,
    expanded: bool,
    can_move_up: bool,
    can_move_down: bool,
    colors: &ThemeColors,
) -> ActionCardResponse {
    let category = ActionCategory::of(action);
    let mut response = ActionCardResponse::None;

    ui.horizontal(|ui| {
        // Action name.
        ui.label(
            egui::RichText::new(action_name(action))
                .family(FontFamily::Name("SemiBold".into()))
                .color(colors.text),
        );

        // Category badge.
        ui.label(
            egui::RichText::new(category.label())
                .monospace()
                .small()
                .color(colors.text_dim),
        );

        // Right-aligned controls.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Delete button.
            let delete_btn = ui.add(
                egui::Button::new(egui::RichText::new("X").small().color(colors.text_dim))
                    .min_size(BUTTON_MIN_SIZE)
                    .frame_when_inactive(false),
            );
            if delete_btn.clicked() {
                response = ActionCardResponse::Delete;
            }
            if delete_btn.hovered() {
                ui.ctx()
                    .output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
            }

            // Move-down arrow.
            response = render_move_button(
                ui,
                "\u{25BC}",
                can_move_down,
                ActionCardResponse::MoveDown,
                response,
                colors,
            );

            // Move-up arrow.
            response = render_move_button(
                ui,
                "\u{25B2}",
                can_move_up,
                ActionCardResponse::MoveUp,
                response,
                colors,
            );

            // Expand/collapse chevron button.
            let chevron = if expanded { "\u{25BC}" } else { "\u{25B6}" };
            let toggle_btn = ui.add(
                egui::Button::new(egui::RichText::new(chevron).small().color(colors.text_dim))
                    .min_size(BUTTON_MIN_SIZE)
                    .frame_when_inactive(false),
            );
            if toggle_btn.clicked() {
                response = ActionCardResponse::Toggle;
            }
            if toggle_btn.hovered() {
                ui.ctx()
                    .output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
            }
        });
    });

    response
}

/// Render a move-up or move-down button, returning the updated response.
///
/// When `enabled` is `false`, the button is rendered in a very dim color
/// and clicks are ignored.
fn render_move_button(
    ui: &mut egui::Ui,
    symbol: &str,
    enabled: bool,
    action: ActionCardResponse,
    current_response: ActionCardResponse,
    colors: &ThemeColors,
) -> ActionCardResponse {
    let text_color = if enabled {
        colors.text_dim
    } else {
        colors.surface1
    };

    let btn = ui.add(
        egui::Button::new(egui::RichText::new(symbol).small().color(text_color))
            .min_size(BUTTON_MIN_SIZE)
            .frame_when_inactive(false),
    );

    if enabled && btn.clicked() {
        return action;
    }
    if enabled && btn.hovered() {
        ui.ctx()
            .output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
    }

    current_response
}

/// Paint the vertical flow connector between two stacked cards.
///
/// Should be called between each pair of adjacent cards to visualize
/// the processing pipeline flow direction.
pub(crate) fn flow_connector(ui: &mut egui::Ui, colors: &ThemeColors) {
    let desired_size = Vec2::new(ACCENT_BAR_WIDTH + 16.0, FLOW_CONNECTOR_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);

    // Center the line on the accent bar column.
    let center_x = rect.left() + ACCENT_BAR_WIDTH * 0.5;
    painter.line_segment(
        [
            Pos2::new(center_x, rect.top()),
            Pos2::new(center_x, rect.bottom()),
        ],
        Stroke::new(FLOW_CONNECTOR_WIDTH, colors.surface1),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_category_classification() {
        use inputforge_core::processing::deadzone::DeadzoneConfig;

        assert_eq!(
            ActionCategory::of(&Action::Invert),
            ActionCategory::Processing,
        );
        assert_eq!(
            ActionCategory::of(&Action::Deadzone {
                config: DeadzoneConfig::default(),
            }),
            ActionCategory::Processing,
        );
        assert_eq!(
            ActionCategory::of(&Action::MapToKeyboard {
                key: inputforge_core::types::KeyCombo {
                    key: "A".to_owned(),
                    modifiers: vec![],
                },
            }),
            ActionCategory::Output,
        );
        assert_eq!(
            ActionCategory::of(&Action::ChangeMode {
                strategy: inputforge_core::action::ModeChangeStrategy::Previous,
            }),
            ActionCategory::Control,
        );
    }

    #[test]
    fn action_names_are_nonempty() {
        let actions: Vec<Action> = vec![
            Action::Invert,
            Action::Deadzone {
                config: inputforge_core::processing::deadzone::DeadzoneConfig::default(),
            },
        ];
        for action in &actions {
            assert!(!action_name(action).is_empty());
        }
    }

    #[test]
    fn action_card_response_variants_are_distinct() {
        assert_ne!(ActionCardResponse::None, ActionCardResponse::Delete);
        assert_ne!(ActionCardResponse::MoveUp, ActionCardResponse::MoveDown);
    }

    #[test]
    fn category_labels_are_uppercase() {
        for cat in [
            ActionCategory::Processing,
            ActionCategory::Output,
            ActionCategory::Control,
        ] {
            let label = cat.label();
            assert_eq!(
                label,
                label.to_uppercase(),
                "category label must be uppercase"
            );
        }
    }
}
