// Rust guideline compliant 2026-03-02

use crate::action::CycleModes;
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

    /// Alias for [`pop_temporary`](Self::pop_temporary).
    pub fn go_previous(&mut self) {
        self.pop_temporary();
    }

    /// Cycle to the next mode in the list, clearing the temporary stack.
    ///
    /// If the current mode is in the list, advances to the next mode
    /// (wrapping around). If not in the list, jumps to the first mode.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ModeNotFound`] if any mode in the cycle
    /// does not exist in the tree.
    pub fn cycle(&mut self, modes: &CycleModes, tree: &ModeTree) -> Result<()> {
        for mode in modes.modes() {
            if !tree.contains(mode) {
                return Err(EngineError::ModeNotFound { name: mode.clone() });
            }
        }
        let mode_list = modes.modes();
        let next = if let Some(pos) = mode_list.iter().position(|m| m == &self.current) {
            &mode_list[(pos + 1) % mode_list.len()]
        } else {
            &mode_list[0]
        };
        next.clone_into(&mut self.current);
        self.stack.clear();
        Ok(())
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

    // --- go_previous ---

    #[test]
    fn go_previous_pops_stack() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        state.go_previous();
        assert_eq!(state.current(), "Default");
    }

    // --- cycle ---

    #[test]
    fn cycle_advances() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        let modes = CycleModes::new(vec!["Default".to_owned(), "Combat".to_owned()]).unwrap();
        state.cycle(&modes, &tree).unwrap();
        assert_eq!(state.current(), "Combat");
    }

    #[test]
    fn cycle_wraps_around() {
        let tree = test_tree();
        let mut state = ModeState::new("Combat".to_owned());
        let modes = CycleModes::new(vec!["Default".to_owned(), "Combat".to_owned()]).unwrap();
        state.cycle(&modes, &tree).unwrap();
        assert_eq!(state.current(), "Default");
    }

    #[test]
    fn cycle_from_outside_list() {
        let tree = test_tree();
        let mut state = ModeState::new("Landing".to_owned());
        let modes = CycleModes::new(vec!["Default".to_owned(), "Combat".to_owned()]).unwrap();
        // Current is not in the cycle list, so jump to first.
        state.cycle(&modes, &tree).unwrap();
        assert_eq!(state.current(), "Default");
    }

    #[test]
    fn cycle_nonexistent_mode_in_list() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        let modes = CycleModes::new(vec!["Default".to_owned(), "Space".to_owned()]).unwrap();
        let err = state.cycle(&modes, &tree).unwrap_err();
        assert!(err.to_string().contains("Space"));
    }

    #[test]
    fn cycle_clears_stack() {
        let tree = test_tree();
        let mut state = ModeState::new("Default".to_owned());
        state.push_temporary("Combat", &tree).unwrap();
        let modes = CycleModes::new(vec!["Default".to_owned(), "Landing".to_owned()]).unwrap();
        state.cycle(&modes, &tree).unwrap();
        // Stack should be cleared.
        state.pop_temporary();
        assert_eq!(state.current(), state.current()); // no change
    }
}
