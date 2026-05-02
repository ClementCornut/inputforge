// Rust guideline compliant 2026-03-02

use crate::action::Mapping;
use crate::types::InputAddress;

use super::ModeTree;

/// Resolve a mapping by walking the mode inheritance chain.
///
/// Given a set of mappings, an input address, and the current mode name,
/// returns the first matching mapping found by walking from the current
/// mode up to the root. Child mappings override parent mappings.
///
/// Returns `None` if no mapping is found in the entire ancestor chain.
#[must_use]
pub fn resolve_mapping<'a>(
    mappings: &'a [Mapping],
    input: &InputAddress,
    mode: &str,
    tree: &ModeTree,
) -> Option<&'a Mapping> {
    let chain = tree.ancestors(mode);
    for ancestor_mode in &chain {
        if let Some(mapping) = mappings
            .iter()
            .find(|m| m.input == *input && m.mode == *ancestor_mode)
        {
            return Some(mapping);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::action::Action;
    use crate::types::{DeviceId, InputId};

    /// Build the standard test tree:
    /// Default -> [Combat -> [Missiles, Guns], Landing]
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

    fn test_input() -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        }
    }

    #[test]
    fn resolve_mapping_direct() {
        let tree = test_tree();
        let input = test_input();
        let mappings = vec![Mapping {
            input: input.clone(),
            mode: "Missiles".to_owned(),
            name: None,
            actions: vec![Action::Invert],
        }];
        let result = resolve_mapping(&mappings, &input, "Missiles", &tree);
        assert!(result.is_some());
        assert_eq!(result.unwrap().mode, "Missiles");
    }

    #[test]
    fn resolve_mapping_inherited() {
        let tree = test_tree();
        let input = test_input();
        let mappings = vec![Mapping {
            input: input.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Invert],
        }];
        // Look up from Missiles, should inherit from Default.
        let result = resolve_mapping(&mappings, &input, "Missiles", &tree);
        assert!(result.is_some());
        assert_eq!(result.unwrap().mode, "Default");
    }

    #[test]
    fn resolve_mapping_not_found() {
        let tree = test_tree();
        let input = test_input();
        let mappings: Vec<Mapping> = vec![];
        let result = resolve_mapping(&mappings, &input, "Missiles", &tree);
        assert!(result.is_none());
    }

    #[test]
    fn resolve_mapping_child_overrides_parent() {
        let tree = test_tree();
        let input = test_input();
        let mappings = vec![
            Mapping {
                input: input.clone(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![Action::Invert],
            },
            Mapping {
                input: input.clone(),
                mode: "Combat".to_owned(),
                name: None,
                actions: vec![], // Different actions to distinguish.
            },
        ];
        // Look up from Combat: should find Combat's mapping, not Default's.
        let result = resolve_mapping(&mappings, &input, "Combat", &tree);
        assert!(result.is_some());
        assert_eq!(result.unwrap().mode, "Combat");
    }

    #[test]
    fn resolve_mapping_wrong_input() {
        let tree = test_tree();
        let input = test_input();
        let other_input = InputAddress::Bound {
            device: DeviceId("dev-2".to_owned()),
            input: InputId::Button { index: 5 },
        };
        let mappings = vec![Mapping {
            input: other_input,
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Invert],
        }];
        let result = resolve_mapping(&mappings, &input, "Default", &tree);
        assert!(result.is_none());
    }

    #[test]
    fn resolve_mapping_nonexistent_mode_returns_none() {
        let tree = test_tree();
        let input = test_input();
        let mappings = vec![Mapping {
            input: input.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Invert],
        }];
        // Mode "Space" doesn't exist, ancestors returns empty, so no match.
        let result = resolve_mapping(&mappings, &input, "Space", &tree);
        assert!(result.is_none());
    }
}
