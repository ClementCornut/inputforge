// Rust guideline compliant 2026-03-04

//! Mapping editor central panel.
//!
//! Displays an editable pipeline of [`Action`] cards for the currently
//! selected input. Actions can be reordered via up/down arrow buttons,
//! added from a categorized dropdown, or deleted individually.

mod card_list;

use std::collections::HashSet;
use std::sync::mpsc;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{InputAddress, InputId};

use crate::app::CachedState;
use crate::theme;

/// Persistent state for the mapping editor panel.
#[derive(Debug, Default)]
pub(crate) struct MappingEditorState {
    /// Working copy of the action pipeline being edited.
    actions: Vec<Action>,
    /// Stable unique ID for each action (parallel to `actions`).
    action_ids: Vec<u64>,
    /// Next unique ID to assign.
    next_id: u64,
    /// Indices of expanded action cards.
    expanded: HashSet<usize>,
    /// Whether the working copy has unsaved changes.
    dirty: bool,
    /// The input address currently being edited, if any.
    editing: Option<InputAddress>,
    /// User-editable name for the mapping.
    mapping_name: String,
}

impl MappingEditorState {
    /// Create a new empty mapping editor state.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Push an action with a stable unique ID.
    fn push_action(&mut self, action: Action) {
        let index = self.actions.len();
        self.actions.push(action);
        self.action_ids.push(self.next_id);
        self.next_id += 1;
        self.expanded.insert(index);
        self.dirty = true;
    }

    /// Load a mapping for editing.
    ///
    /// Replaces the current pipeline, resets dirty state, and assigns
    /// fresh stable IDs to each action.
    pub(crate) fn load(
        &mut self,
        address: InputAddress,
        name: Option<String>,
        actions: Vec<Action>,
    ) {
        self.editing = Some(address);
        self.mapping_name = name.unwrap_or_default();
        self.action_ids = (0..actions.len() as u64).collect();
        self.next_id = actions.len() as u64;
        self.actions = actions;
        self.expanded.clear();
        self.dirty = false;
    }

    /// The input address currently being edited.
    pub(crate) fn editing(&self) -> Option<&InputAddress> {
        self.editing.as_ref()
    }

    /// Whether the working copy has unsaved changes.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the editor as having unsaved changes.
    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark the editor as clean (no unsaved changes).
    pub(crate) fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Clone the current actions for saving.
    pub(crate) fn take_actions(&self) -> Vec<Action> {
        self.actions.clone()
    }

    /// Return the mapping name if non-empty.
    pub(crate) fn take_name(&self) -> Option<String> {
        if self.mapping_name.is_empty() {
            None
        } else {
            Some(self.mapping_name.clone())
        }
    }

    /// The current mapping name (for UI binding).
    pub(crate) fn mapping_name_mut(&mut self) -> &mut String {
        &mut self.mapping_name
    }

    /// Reset to empty/no-input state.
    pub(crate) fn clear(&mut self) {
        self.editing = None;
        self.mapping_name.clear();
        self.actions.clear();
        self.action_ids.clear();
        self.next_id = 0;
        self.expanded.clear();
        self.dirty = false;
    }
}

/// Render the mapping editor panel.
///
/// If no device or input is selected, shows a placeholder message.
/// Otherwise displays the action pipeline with arrow-button
/// reordering, add/delete controls, and per-action configuration.
/// Returns `true` when the user clicks "Discard", signalling that the
/// caller should reload the current mapping from the saved profile.
pub(crate) fn show(
    ui: &mut egui::Ui,
    state: &mut MappingEditorState,
    cache: &CachedState,
    commands: &mpsc::Sender<EngineCommand>,
) -> bool {
    let colors = theme::colors(ui.ctx());

    // If no input is selected, show placeholder.
    let Some(editing_addr) = state.editing() else {
        crate::widgets::empty_state::empty_state(ui, "Select an input in the device tree to begin");
        return false;
    };
    let editing_addr = editing_addr.clone(); // Clone to release borrow on state

    // Header: input name
    let input_label = match &editing_addr.input {
        InputId::Axis { index } => {
            format!(
                "Axis {}",
                crate::panels::device_view::axis_label(usize::from(*index))
            )
        }
        InputId::Button { index } => format!("Button {}", index + 1),
        InputId::Hat { index } => format!("Hat {index}"),
    };
    ui.heading(egui::RichText::new(&input_label).color(colors.text));

    // Editable name field
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Name:").color(colors.text_dim));
        let name_response = ui.add(
            egui::TextEdit::singleline(state.mapping_name_mut())
                .hint_text("(optional)")
                .desired_width(200.0),
        );
        if name_response.changed() {
            state.mark_dirty();
        }
    });

    // Save / Discard buttons
    let mut discard_clicked = false;
    ui.horizontal(|ui| {
        let save_btn = ui.add_enabled(state.is_dirty(), egui::Button::new("Save"));
        if save_btn.clicked() {
            let _ = commands.send(EngineCommand::SetMapping {
                input: editing_addr.clone(),
                mode: cache.current_mode.clone(),
                name: state.take_name(),
                actions: state.take_actions(),
            });
            state.mark_clean();
        }

        let discard_btn = ui.add_enabled(state.is_dirty(), egui::Button::new("Discard"));
        if discard_btn.clicked() {
            discard_clicked = true;
        }
    });

    if discard_clicked {
        return true;
    }

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // Scrollable action card list.
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            card_list::show_action_list(ui, state, cache, colors);
            ui.add_space(8.0);
            card_list::show_add_action_dropdown(ui, state, colors);
        });

    false
}

/// Update the expanded set after swapping two adjacent actions.
pub(super) fn reindex_expanded_after_swap(
    expanded: &mut HashSet<usize>,
    index_a: usize,
    index_b: usize,
) {
    let a_was_expanded = expanded.remove(&index_a);
    let b_was_expanded = expanded.remove(&index_b);
    if a_was_expanded {
        expanded.insert(index_b);
    }
    if b_was_expanded {
        expanded.insert(index_a);
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
        // Both swapped, both remain expanded at swapped positions.
        assert!(expanded.contains(&1));
        assert!(expanded.contains(&2));
    }

    #[test]
    fn load_sets_editing_and_actions() {
        use inputforge_core::types::{DeviceId, InputId};

        let mut state = MappingEditorState::new();
        let addr = InputAddress {
            device: DeviceId("dev-0".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let actions = vec![Action::Invert, Action::Invert];
        state.load(addr.clone(), Some("Roll".to_owned()), actions);

        assert_eq!(state.editing(), Some(&addr));
        assert_eq!(state.mapping_name, "Roll");
        assert_eq!(state.actions.len(), 2);
        assert_eq!(state.action_ids, vec![0, 1]);
        assert_eq!(state.next_id, 2);
        assert!(!state.dirty);
        assert!(state.expanded.is_empty());
    }

    #[test]
    fn clear_resets_all_state() {
        use inputforge_core::types::{DeviceId, InputId};

        let mut state = MappingEditorState::new();
        let addr = InputAddress {
            device: DeviceId("dev-0".to_owned()),
            input: InputId::Button { index: 1 },
        };
        state.load(addr, Some("Fire".to_owned()), vec![Action::Invert]);
        state.clear();

        assert!(state.editing().is_none());
        assert!(state.mapping_name.is_empty());
        assert!(state.actions.is_empty());
        assert!(state.action_ids.is_empty());
        assert_eq!(state.next_id, 0);
        assert!(state.expanded.is_empty());
        assert!(!state.dirty);
    }

    #[test]
    fn take_name_empty_returns_none() {
        let state = MappingEditorState::new();
        assert!(state.take_name().is_none());
    }

    #[test]
    fn take_name_nonempty_returns_some() {
        use inputforge_core::types::{DeviceId, InputId};

        let mut state = MappingEditorState::new();
        let addr = InputAddress {
            device: DeviceId("dev-0".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        state.load(addr, Some("Roll".to_owned()), vec![]);
        assert_eq!(state.take_name(), Some("Roll".to_owned()));
    }

    #[test]
    fn dirty_flag_tracks_mutations() {
        use inputforge_core::types::{DeviceId, InputId};

        let mut state = MappingEditorState::new();
        assert!(!state.is_dirty());

        state.push_action(Action::Invert);
        assert!(state.is_dirty());

        let addr = InputAddress {
            device: DeviceId("dev-0".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        state.load(addr, None, vec![]);
        assert!(!state.is_dirty());
    }
}
