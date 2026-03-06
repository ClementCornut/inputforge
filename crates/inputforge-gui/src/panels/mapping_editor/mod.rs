// Rust guideline compliant 2026-03-04

//! Mapping editor central panel.
//!
//! Displays an editable pipeline of [`Action`] cards for the currently
//! selected input. Actions can be reordered via up/down arrow buttons,
//! added from a categorized dropdown, or deleted individually.

mod card_list;

use std::collections::HashSet;

use inputforge_core::action::Action;

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
}

/// Render the mapping editor panel.
///
/// If no device or input is selected, shows a placeholder message.
/// Otherwise displays the action pipeline with arrow-button
/// reordering, add/delete controls, and per-action configuration.
pub(crate) fn show(ui: &mut egui::Ui, state: &mut MappingEditorState, cache: &CachedState) {
    let colors = theme::colors(ui.ctx());

    // Scrollable action card list.
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            card_list::show_action_list(ui, state, cache, colors);

            ui.add_space(8.0);

            // "Add Action" dropdown at the bottom.
            card_list::show_add_action_dropdown(ui, state, colors);
        });
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
        // Both swapped — both remain expanded at swapped positions.
        assert!(expanded.contains(&1));
        assert!(expanded.contains(&2));
    }
}
