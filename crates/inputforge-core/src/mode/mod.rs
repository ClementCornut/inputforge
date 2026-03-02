// Rust guideline compliant 2026-03-02

mod resolve;
mod state;

pub use resolve::resolve_mapping;
pub use state::ModeState;

use std::collections::{HashMap, HashSet};

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

/// A node in the mode tree.
///
/// Each node has a name and zero or more child nodes. Nodes with no
/// children are leaf modes.
#[derive(Debug, Clone, PartialEq)]
pub struct ModeNode {
    name: String,
    children: Vec<ModeNode>,
}

impl ModeNode {
    /// Return the mode name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the child nodes.
    #[must_use]
    pub fn children(&self) -> &[ModeNode] {
        &self.children
    }
}

/// A tree of input modes with parent-child inheritance.
///
/// The tree has exactly one root node. Child modes inherit unmapped
/// inputs from their parent, walking up to the root.
///
/// # Serialization
///
/// Serializes as a flat adjacency map where keys are parent mode names
/// and values are lists of child names. Leaf modes that appear only as
/// children do not need their own key entries.
///
/// ```toml
/// [modes]
/// Default = ["Combat", "Landing"]
/// Combat = ["Missiles", "Guns"]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModeTree {
    root: ModeNode,
}

impl ModeTree {
    /// Build a mode tree from a flat adjacency map.
    ///
    /// Keys are parent mode names, values are lists of child mode names.
    /// The root is auto-detected as the key that never appears as any
    /// other key's child.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] when:
    /// - The map is empty
    /// - No root can be determined (every key appears as some child)
    /// - Multiple roots exist (more than one key is never a child)
    /// - Duplicate mode names exist
    /// - A child references a mode name that appears nowhere
    pub fn from_adjacency(map: &HashMap<String, Vec<String>>) -> Result<Self> {
        if map.is_empty() {
            return Err(EngineError::InvalidConfig {
                reason: "mode tree cannot be empty".to_owned(),
            });
        }

        // Collect all mode names that appear as children, detecting
        // duplicates (a child appearing under multiple parents).
        let mut all_children: HashSet<&str> = HashSet::new();
        for children in map.values() {
            for child in children {
                if !all_children.insert(child.as_str()) {
                    return Err(EngineError::InvalidConfig {
                        reason: format!("duplicate mode name: {child}"),
                    });
                }
            }
        }

        // Collect all unique mode names (keys + values).
        let mut all_names: HashSet<&str> = HashSet::new();
        for key in map.keys() {
            all_names.insert(key.as_str());
        }
        for name in &all_children {
            all_names.insert(name);
        }

        // Roots are keys that never appear in any value list.
        let roots: Vec<&str> = map
            .keys()
            .filter(|k| !all_children.contains(k.as_str()))
            .map(String::as_str)
            .collect();

        if roots.is_empty() {
            return Err(EngineError::InvalidConfig {
                reason: "no root mode found (every key appears as a child)".to_owned(),
            });
        }
        if roots.len() > 1 {
            return Err(EngineError::InvalidConfig {
                reason: format!("multiple root modes found: {}", roots.join(", ")),
            });
        }

        let root_name = roots[0];
        let root = build_node(root_name, map)?;

        // Verify all modes are reachable from root.
        let mut reachable = HashSet::new();
        collect_names(&root, &mut reachable);

        if reachable.len() != all_names.len() {
            let unreachable: Vec<&&str> = all_names
                .iter()
                .filter(|n| !reachable.contains(**n))
                .collect();
            return Err(EngineError::InvalidConfig {
                reason: format!(
                    "unreachable modes: {}",
                    unreachable
                        .iter()
                        .map(|n| (**n).to_owned())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
        }

        Ok(Self { root })
    }

    /// Return the root node.
    #[must_use]
    pub fn root(&self) -> &ModeNode {
        &self.root
    }

    /// Find a mode node by name (recursive DFS).
    #[must_use]
    pub fn find_mode(&self, name: &str) -> Option<&ModeNode> {
        find_node(&self.root, name)
    }

    /// Return the ancestor chain from the named mode up to the root.
    ///
    /// For a tree `Default -> Combat -> Missiles`, calling
    /// `ancestors("Missiles")` returns `["Missiles", "Combat", "Default"]`.
    ///
    /// Returns an empty vec if the mode is not found.
    #[must_use]
    pub fn ancestors(&self, name: &str) -> Vec<&str> {
        ancestors_helper(&self.root, name).unwrap_or_default()
    }

    /// Check whether a mode with the given name exists in the tree.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        find_node(&self.root, name).is_some()
    }

    /// Return a flat list of all mode names in the tree.
    #[must_use]
    pub fn all_modes(&self) -> Vec<&str> {
        let mut names = Vec::new();
        collect_names_vec(&self.root, &mut names);
        names
    }

    /// Convert the tree back to a flat adjacency map for serialization.
    fn to_adjacency_map(&self) -> HashMap<&str, Vec<&str>> {
        let mut map = HashMap::new();
        build_adjacency_map(&self.root, &mut map, true);
        map
    }
}

// --- Private helpers ---

/// Recursively build a `ModeNode` from the adjacency map.
fn build_node(name: &str, map: &HashMap<String, Vec<String>>) -> Result<ModeNode> {
    let children = if let Some(child_names) = map.get(name) {
        child_names
            .iter()
            .map(|child_name| build_node(child_name, map))
            .collect::<Result<Vec<_>>>()?
    } else {
        // Leaf mode: not a key in the map, just appears as a child value.
        Vec::new()
    };
    Ok(ModeNode {
        name: name.to_owned(),
        children,
    })
}

/// Recursively search for a node by name.
fn find_node<'a>(node: &'a ModeNode, name: &str) -> Option<&'a ModeNode> {
    if node.name == name {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node(child, name) {
            return Some(found);
        }
    }
    None
}

/// Return the ancestor path from the target node up to (and including)
/// the given node, or `None` if the target is not a descendant.
fn ancestors_helper<'a>(node: &'a ModeNode, name: &str) -> Option<Vec<&'a str>> {
    if node.name == name {
        return Some(vec![node.name.as_str()]);
    }
    for child in &node.children {
        if let Some(mut path) = ancestors_helper(child, name) {
            path.push(node.name.as_str());
            return Some(path);
        }
    }
    None
}

/// Collect all mode names reachable from a node into a `HashSet`.
fn collect_names<'a>(node: &'a ModeNode, names: &mut HashSet<&'a str>) {
    names.insert(node.name.as_str());
    for child in &node.children {
        collect_names(child, names);
    }
}

/// Collect all mode names reachable from a node into a `Vec`.
fn collect_names_vec<'a>(node: &'a ModeNode, names: &mut Vec<&'a str>) {
    names.push(node.name.as_str());
    for child in &node.children {
        collect_names_vec(child, names);
    }
}

/// Recursively build the adjacency map from the tree.
///
/// The root is always included (even with an empty child list) so that
/// a single-mode tree roundtrips correctly. Internal nodes with children
/// are also included. Leaf nodes that are not the root are omitted.
fn build_adjacency_map<'a>(
    node: &'a ModeNode,
    map: &mut HashMap<&'a str, Vec<&'a str>>,
    is_root: bool,
) {
    if is_root || !node.children.is_empty() {
        let child_names: Vec<&str> = node.children.iter().map(|c| c.name.as_str()).collect();
        map.insert(node.name.as_str(), child_names);
    }
    for child in &node.children {
        build_adjacency_map(child, map, false);
    }
}

// --- Serde ---

impl Serialize for ModeTree {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let adjacency = self.to_adjacency_map();
        // Serialize as a map of string -> array of strings.
        let mut map = serializer.serialize_map(Some(adjacency.len()))?;
        // Serialize root first for deterministic output, then children.
        serialize_node_map(&self.root, &adjacency, &mut map)?;
        map.end()
    }
}

/// Serialize the adjacency map entries in tree order (root first, DFS).
fn serialize_node_map<S: SerializeMap>(
    node: &ModeNode,
    adjacency: &HashMap<&str, Vec<&str>>,
    map: &mut S,
) -> std::result::Result<(), S::Error> {
    if let Some(children) = adjacency.get(node.name.as_str()) {
        map.serialize_entry(node.name.as_str(), children)?;
        for child in &node.children {
            serialize_node_map(child, adjacency, map)?;
        }
    }
    Ok(())
}

impl<'de> Deserialize<'de> for ModeTree {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = HashMap::<String, Vec<String>>::deserialize(deserializer)?;
        Self::from_adjacency(&raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // --- Tree construction ---

    #[test]
    fn from_adjacency_builds_tree() {
        let tree = test_tree();
        assert_eq!(tree.root().name(), "Default");
        assert_eq!(tree.root().children().len(), 2);
        assert_eq!(tree.root().children()[0].name(), "Combat");
        assert_eq!(tree.root().children()[1].name(), "Landing");
        assert_eq!(tree.root().children()[0].children().len(), 2);
        assert_eq!(tree.root().children()[0].children()[0].name(), "Missiles");
        assert_eq!(tree.root().children()[0].children()[1].name(), "Guns");
        assert!(tree.root().children()[1].children().is_empty());
    }

    #[test]
    fn single_mode_tree() {
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let tree = ModeTree::from_adjacency(&map).unwrap();
        assert_eq!(tree.root().name(), "Default");
        assert!(tree.root().children().is_empty());
    }

    // --- find_mode ---

    #[test]
    fn find_mode_root() {
        let tree = test_tree();
        let node = tree.find_mode("Default").unwrap();
        assert_eq!(node.name(), "Default");
    }

    #[test]
    fn find_mode_leaf() {
        let tree = test_tree();
        let node = tree.find_mode("Missiles").unwrap();
        assert_eq!(node.name(), "Missiles");
    }

    #[test]
    fn find_mode_middle() {
        let tree = test_tree();
        let node = tree.find_mode("Combat").unwrap();
        assert_eq!(node.name(), "Combat");
    }

    #[test]
    fn find_mode_nonexistent() {
        let tree = test_tree();
        assert!(tree.find_mode("Space").is_none());
    }

    // --- contains ---

    #[test]
    fn contains_existing() {
        let tree = test_tree();
        assert!(tree.contains("Combat"));
        assert!(tree.contains("Guns"));
        assert!(tree.contains("Default"));
    }

    #[test]
    fn contains_nonexistent() {
        let tree = test_tree();
        assert!(!tree.contains("Space"));
    }

    // --- ancestors ---

    #[test]
    fn ancestors_root() {
        let tree = test_tree();
        assert_eq!(tree.ancestors("Default"), vec!["Default"]);
    }

    #[test]
    fn ancestors_leaf() {
        let tree = test_tree();
        assert_eq!(
            tree.ancestors("Missiles"),
            vec!["Missiles", "Combat", "Default"]
        );
    }

    #[test]
    fn ancestors_middle() {
        let tree = test_tree();
        assert_eq!(tree.ancestors("Combat"), vec!["Combat", "Default"]);
    }

    #[test]
    fn ancestors_other_leaf() {
        let tree = test_tree();
        assert_eq!(tree.ancestors("Landing"), vec!["Landing", "Default"]);
    }

    #[test]
    fn ancestors_nonexistent() {
        let tree = test_tree();
        let result: Vec<&str> = tree.ancestors("Space");
        assert!(result.is_empty());
    }

    // --- all_modes ---

    #[test]
    fn all_modes_returns_all() {
        let tree = test_tree();
        let modes = tree.all_modes();
        assert_eq!(modes.len(), 5);
        assert!(modes.contains(&"Default"));
        assert!(modes.contains(&"Combat"));
        assert!(modes.contains(&"Missiles"));
        assert!(modes.contains(&"Guns"));
        assert!(modes.contains(&"Landing"));
    }

    // --- Serde ---

    #[test]
    fn serde_toml_roundtrip() {
        let tree = test_tree();
        let toml_str = toml::to_string(&tree).unwrap();
        let back: ModeTree = toml::from_str(&toml_str).unwrap();
        assert_eq!(tree, back);
    }

    #[test]
    fn serde_json_roundtrip() {
        let tree = test_tree();
        let json = serde_json::to_string(&tree).unwrap();
        let back: ModeTree = serde_json::from_str(&json).unwrap();
        assert_eq!(tree, back);
    }

    #[test]
    fn single_mode_serde_roundtrip() {
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let tree = ModeTree::from_adjacency(&map).unwrap();
        let toml_str = toml::to_string(&tree).unwrap();
        let back: ModeTree = toml::from_str(&toml_str).unwrap();
        assert_eq!(tree, back);
    }

    // --- Validation ---

    #[test]
    fn reject_empty_map() {
        let map = HashMap::new();
        let err = ModeTree::from_adjacency(&map).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn reject_multiple_roots() {
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        map.insert("Other".to_owned(), vec![]);
        let err = ModeTree::from_adjacency(&map).unwrap_err();
        assert!(err.to_string().contains("multiple root"));
    }

    #[test]
    fn reject_no_root() {
        // Both modes reference each other as children (cycle).
        let mut map = HashMap::new();
        map.insert("A".to_owned(), vec!["B".to_owned()]);
        map.insert("B".to_owned(), vec!["A".to_owned()]);
        let err = ModeTree::from_adjacency(&map).unwrap_err();
        // Both appear as children, so no root is found.
        assert!(err.to_string().contains("no root"));
    }

    #[test]
    fn reject_child_appears_under_multiple_parents() {
        let mut map = HashMap::new();
        map.insert("Root".to_owned(), vec!["A".to_owned(), "Shared".to_owned()]);
        map.insert("A".to_owned(), vec!["Shared".to_owned()]);
        let err = ModeTree::from_adjacency(&map).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn reject_duplicate_key_child_overlap() {
        // A mode appears both as a key and as a child of another key
        // is normal (it's an internal node). But if it appears as a
        // child of TWO different parents, that's a duplicate.
        let mut map = HashMap::new();
        map.insert("Root".to_owned(), vec!["A".to_owned()]);
        map.insert("A".to_owned(), vec!["B".to_owned()]);
        // This is fine: A is child of Root and also a key. No duplication.
        let tree = ModeTree::from_adjacency(&map);
        tree.unwrap();
    }

    #[test]
    fn reject_serde_invalid_json() {
        let result: std::result::Result<ModeTree, _> = serde_json::from_str("{}");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }
}
