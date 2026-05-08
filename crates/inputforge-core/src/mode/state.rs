// Rust guideline compliant 2026-03-02

use crate::error::{EngineError, Result};

use super::ModeTree;

/// Runtime mode state machine.
///
/// Tracks the current mode and a stack of temporary modes. The stack
/// enables "hold to activate" patterns where releasing the input pops
/// back to the previous mode.
#[derive(Debug, Clone)]
pub struct ModeState {
    current: String,
    stack: Vec<String>,
}

impl ModeState {
    /// Create a new mode state with the given initial mode.
    #[must_use]
    pub fn new(initial: String) -> Self {
        Self {
            current: initial,
            stack: Vec::new(),
        }
    }

    /// Return the current mode name.
    #[must_use]
    pub fn current(&self) -> &str {
        &self.current
    }

    /// Switch to a named mode, clearing the temporary stack.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ModeNotFound`] if the mode does not exist
    /// in the tree.
    pub fn switch_to(&mut self, name: &str, tree: &ModeTree) -> Result<()> {
        if !tree.contains(name) {
            return Err(EngineError::ModeNotFound {
                name: name.to_owned(),
            });
        }
        name.clone_into(&mut self.current);
        self.stack.clear();
        Ok(())
    }

    /// Push a temporary mode onto the stack.
    ///
    /// The current mode is saved on the stack and can be restored with
    /// [`pop_temporary`](Self::pop_temporary).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ModeNotFound`] if the mode does not exist
    /// in the tree, or [`EngineError::ModeCycleDetected`] if the mode
    /// is already on the stack or equals the current mode.
    pub fn push_temporary(&mut self, name: &str, tree: &ModeTree) -> Result<()> {
        if !tree.contains(name) {
            return Err(EngineError::ModeNotFound {
                name: name.to_owned(),
            });
        }
        if name == self.current || self.stack.iter().any(|s| s == name) {
            let mut path = self.stack.clone();
            path.push(self.current.clone());
            path.push(name.to_owned());
            return Err(EngineError::ModeCycleDetected { path });
        }
        self.stack.push(self.current.clone());
        name.clone_into(&mut self.current);
        Ok(())
    }

    /// Pop the temporary stack, restoring the previous mode.
    ///
    /// No-op if the stack is empty.
    pub fn pop_temporary(&mut self) {
        if let Some(prev) = self.stack.pop() {
            self.current = prev;
        }
    }

    /// Drop every stack entry whose name is in `removed`. Used by
    /// [`EngineCommand::DeleteMode`] cascade.
    pub fn clear_stack_entries(&mut self, removed: &[String]) {
        self.stack
            .retain(|entry| !removed.iter().any(|r| r == entry));
    }

    /// Rewrite every entry equal to `from` to `to`, both `current` and the
    /// temporary stack. Used by [`EngineCommand::RenameMode`] cascade.
    pub fn rename_in_place(&mut self, from: &str, to: &str) {
        if self.current == from {
            to.clone_into(&mut self.current);
        }
        for entry in &mut self.stack {
            if entry == from {
                to.clone_into(entry);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn test_tree() -> ModeTree {
        let mut map = HashMap::new();
        map.insert(
            "Default".to_owned(),
            vec!["Combat".to_owned(), "Landing".to_owned()],
        );
        map.insert(
            "Combat".to_owned(),
            vec!["Missiles".to_owned(), "Guns".to_owned()],
        );
        ModeTree::from_adjacency(&map).unwrap()
    }

    // --- new / current ---

    #[test]
    fn initial_mode() {
        let state = ModeState::new("Default".to_owned());
        assert_eq!(state.current(), "Default");
    }

    // --- switch_to ---

    #[test]
    fn switch_to_valid_mode() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.switch_to("Combat", &tree).unwrap();
        assert_eq!(state.current(), "Combat");
    }

    #[test]
    fn switch_to_nonexistent_mode() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        let err = state.switch_to("Space", &tree).unwrap_err();
        assert!(err.to_string().contains("Space"));
    }

    #[test]
    fn switch_to_clears_stack() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        assert_eq!(state.current(), "Combat");
        state.switch_to("Landing", &tree).unwrap();
        assert_eq!(state.current(), "Landing");
        // Pop should be no-op since stack was cleared.
        state.pop_temporary();
        assert_eq!(state.current(), "Landing");
    }

    // --- push_temporary / pop_temporary ---

    #[test]
    fn push_and_pop_temporary() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        assert_eq!(state.current(), "Combat");
        state.pop_temporary();
        assert_eq!(state.current(), "Default");
    }

    #[test]
    fn nested_temporaries() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        state.push_temporary("Missiles", &tree).unwrap();
        assert_eq!(state.current(), "Missiles");
        state.pop_temporary();
        assert_eq!(state.current(), "Combat");
        state.pop_temporary();
        assert_eq!(state.current(), "Default");
    }

    #[test]
    fn pop_empty_stack_is_noop() {
        let mut state = ModeState::new("Default".to_owned());
        state.pop_temporary();
        assert_eq!(state.current(), "Default");
    }

    #[test]
    fn push_nonexistent_mode() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        let err = state.push_temporary("Space", &tree).unwrap_err();
        assert!(err.to_string().contains("Space"));
        // State unchanged.
        assert_eq!(state.current(), "Default");
    }

    #[test]
    fn push_duplicate_detected() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        state.push_temporary("Missiles", &tree).unwrap();
        // Try to push Default again (it's on the stack).
        let err = state.push_temporary("Default", &tree).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn push_same_as_current() {
        let tree = test_tree();
        let mut state = ModeState::new("Combat".to_owned());
        let err = state.push_temporary("Combat", &tree).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    // --- rename_in_place ---

    #[test]
    fn rename_in_place_rewrites_current_and_stack() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        state.rename_in_place("Combat", "Fighter");
        assert_eq!(state.current(), "Fighter");
        state.pop_temporary();
        assert_eq!(state.current(), "Default");
    }

    #[test]
    fn rename_in_place_no_match_is_no_op() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        state.rename_in_place("Nope", "Whatever");
        assert_eq!(state.current(), "Combat");
    }

    // --- clear_stack_entries ---

    #[test]
    fn clear_stack_entries_drops_named() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        state.clear_stack_entries(&["Default".to_owned()]);
        assert_eq!(state.current(), "Combat");
        state.pop_temporary();
        // Stack now empty (Default was dropped). pop_temporary on empty stack is no-op.
        assert_eq!(state.current(), "Combat");
    }
}
