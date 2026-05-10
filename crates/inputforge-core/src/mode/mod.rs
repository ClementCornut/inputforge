// Rust guideline compliant 2026-05-10

mod resolve;
mod state;

pub use resolve::resolve_mapping;
pub use state::ModeState;

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

/// Ordered flat list of profile modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Modes(Vec<String>);

/// Compatibility name for callers not yet migrated to [`Modes`].
pub type ModeTree = Modes;

/// Borrowed mode name returned by legacy tree-shaped APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeNode<'a> {
    name: &'a str,
}

impl<'a> ModeNode<'a> {
    /// Return the mode name.
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }
}

impl Modes {
    /// Create a validated list of modes.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] if the list is empty or if two
    /// names compare equal under ASCII case folding.
    pub fn new(names: Vec<String>) -> Result<Self> {
        if names.is_empty() {
            return Err(EngineError::InvalidConfig {
                reason: "modes cannot be empty".to_owned(),
            });
        }
        reject_duplicate_names(&names)?;
        Ok(Self(names))
    }

    /// Build modes from the legacy adjacency representation.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] if no unique root can be found,
    /// the map is empty, or duplicate names are present.
    pub fn from_adjacency(map: &HashMap<String, Vec<String>>) -> Result<Self> {
        if map.is_empty() {
            return Err(EngineError::InvalidConfig {
                reason: "modes cannot be empty".to_owned(),
            });
        }

        let mut children = HashSet::new();
        for child in map.values().flatten() {
            if !children.insert(child.as_str()) {
                return Err(EngineError::InvalidConfig {
                    reason: format!("duplicate mode name: {child}"),
                });
            }
        }

        let roots: Vec<&str> = map
            .keys()
            .filter(|name| !children.contains(name.as_str()))
            .map(String::as_str)
            .collect();
        let [root] = roots.as_slice() else {
            return Err(EngineError::InvalidConfig {
                reason: "modes must have exactly one root".to_owned(),
            });
        };

        let mut names = Vec::new();
        collect_adjacency_names(root, map, &mut names);
        Self::new(names)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    #[must_use]
    pub fn first(&self) -> &str {
        &self.0[0]
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|candidate| candidate == name)
    }

    #[must_use]
    pub fn root(&self) -> ModeNode<'_> {
        ModeNode { name: self.first() }
    }

    #[must_use]
    pub fn all_modes(&self) -> Vec<&str> {
        self.0.iter().map(String::as_str).collect()
    }

    #[must_use]
    pub fn ancestors(&self, name: &str) -> Vec<&str> {
        if let Some(existing) = self.0.iter().find(|candidate| candidate.as_str() == name) {
            vec![existing.as_str()]
        } else {
            Vec::new()
        }
    }

    /// Return flat-mode descendants for legacy tree callers.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ModeNotFound`] if `name` is not in the list.
    pub fn descendants_of(&self, name: &str) -> Result<Vec<String>> {
        if !self.contains(name) {
            return Err(EngineError::ModeNotFound {
                name: name.to_owned(),
            });
        }
        Ok(Vec::new())
    }

    pub fn with_appended(&self, name: &str) -> Result<Self> {
        let mut names = self.0.clone();
        names.push(name.to_owned());
        Self::new(names)
    }

    pub fn with_added_child(&self, parent: &str, name: &str) -> Result<Self> {
        if !self.contains(parent) {
            return Err(EngineError::ModeNotFound {
                name: parent.to_owned(),
            });
        }
        self.with_appended(name)
    }

    pub fn with_renamed(&self, from: &str, to: &str) -> Result<Self> {
        let Some(index) = self.0.iter().position(|name| name == from) else {
            return Err(EngineError::ModeNotFound {
                name: from.to_owned(),
            });
        };

        let mut names = self.0.clone();
        names[index] = to.to_owned();
        Self::new(names)
    }

    pub fn with_removed(&self, name: &str) -> Result<Self> {
        let Some(index) = self.0.iter().position(|candidate| candidate == name) else {
            return Err(EngineError::ModeNotFound {
                name: name.to_owned(),
            });
        };
        if self.0.len() == 1 {
            return Err(EngineError::InvalidConfig {
                reason: "cannot remove the last mode".to_owned(),
            });
        }

        let mut names = self.0.clone();
        names.remove(index);
        Self::new(names)
    }

    pub fn with_subtree_removed(&self, name: &str) -> Result<Self> {
        self.with_removed(name)
    }
}

fn reject_duplicate_names(names: &[String]) -> Result<()> {
    for (index, name) in names.iter().enumerate() {
        if names[..index]
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(name))
        {
            return Err(EngineError::InvalidConfig {
                reason: format!("duplicate mode name: {name}"),
            });
        }
    }
    Ok(())
}

fn collect_adjacency_names(
    name: &str,
    map: &HashMap<String, Vec<String>>,
    names: &mut Vec<String>,
) {
    names.push(name.to_owned());
    if let Some(children) = map.get(name) {
        for child in children {
            collect_adjacency_names(child, map, names);
        }
    }
}

impl<'de> Deserialize<'de> for Modes {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        let toml::Value::Array(values) = value else {
            return Err(serde::de::Error::custom(
                "modes must be a flat list of strings",
            ));
        };

        let names = values
            .into_iter()
            .map(|value| match value {
                toml::Value::String(name) => Ok(name),
                other => Err(serde::de::Error::custom(format!(
                    "mode names must be strings, found {other:?}"
                ))),
            })
            .collect::<std::result::Result<Vec<_>, D::Error>>()?;

        Self::new(names).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modes(names: &[&str]) -> Modes {
        Modes::new(names.iter().map(|name| (*name).to_owned()).collect()).unwrap()
    }

    #[test]
    fn new_accepts_non_empty_unique_names() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        assert_eq!(modes.as_slice(), ["Default", "Combat", "Landing"]);
        assert_eq!(modes.first(), "Default");
        assert_eq!(modes.len(), 3);
        assert!(modes.contains("Combat"));
        assert!(!modes.contains("Missing"));
    }

    #[test]
    fn new_rejects_empty_list() {
        let err = Modes::new(Vec::new()).unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(err.to_string(), "invalid config: modes cannot be empty");
    }

    #[test]
    fn new_rejects_duplicate_names_case_insensitively() {
        let err = Modes::new(vec!["Combat".to_owned(), "combat".to_owned()]).unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(
            err.to_string(),
            "invalid config: duplicate mode name: combat"
        );
    }

    #[test]
    fn with_appended_places_new_name_at_tail() {
        let modes = modes(&["Default", "Combat"]);

        let modes = modes.with_appended("Landing").unwrap();

        assert_eq!(modes.as_slice(), ["Default", "Combat", "Landing"]);
    }

    #[test]
    fn with_appended_rejects_duplicate_name_case_insensitively() {
        let modes = modes(&["Default", "Combat"]);

        let err = modes.with_appended("combat").unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(
            err.to_string(),
            "invalid config: duplicate mode name: combat"
        );
    }

    #[test]
    fn with_renamed_rewrites_one_name_in_place() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        let modes = modes.with_renamed("Combat", "Cruise").unwrap();

        assert_eq!(modes.as_slice(), ["Default", "Cruise", "Landing"]);
    }

    #[test]
    fn with_renamed_keeps_no_op_rename_valid() {
        let modes = modes(&["Default", "Combat"]);

        let renamed = modes.with_renamed("Combat", "Combat").unwrap();

        assert_eq!(renamed.as_slice(), ["Default", "Combat"]);
    }

    #[test]
    fn with_renamed_rejects_unknown_source() {
        let modes = modes(&["Default", "Combat"]);

        let err = modes.with_renamed("Missing", "Cruise").unwrap_err();

        assert!(matches!(err, EngineError::ModeNotFound { .. }));
    }

    #[test]
    fn with_renamed_rejects_collision_case_insensitively() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        let err = modes.with_renamed("Landing", "combat").unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(
            err.to_string(),
            "invalid config: duplicate mode name: combat"
        );
    }

    #[test]
    fn with_removed_drops_one_name() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        let modes = modes.with_removed("Combat").unwrap();

        assert_eq!(modes.as_slice(), ["Default", "Landing"]);
    }

    #[test]
    fn with_removed_rejects_unknown_name() {
        let modes = modes(&["Default", "Combat"]);

        let err = modes.with_removed("Missing").unwrap_err();

        assert!(matches!(err, EngineError::ModeNotFound { .. }));
    }

    #[test]
    fn with_removed_rejects_last_mode() {
        let modes = modes(&["Default"]);

        let err = modes.with_removed("Default").unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(
            err.to_string(),
            "invalid config: cannot remove the last mode"
        );
    }

    #[test]
    fn descendants_of_is_empty_for_flat_modes_and_rejects_unknown_name() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        let descendants = modes.descendants_of("Combat").unwrap();
        assert!(descendants.is_empty());

        let err = modes.descendants_of("Missing").unwrap_err();
        assert!(matches!(err, EngineError::ModeNotFound { .. }));
    }

    #[test]
    fn serde_toml_roundtrip_uses_flat_list() {
        #[derive(Debug, Deserialize, PartialEq, Serialize)]
        struct Wrapper {
            modes: Modes,
        }

        let wrapper = Wrapper {
            modes: modes(&["Default", "Combat", "Landing"]),
        };

        let toml = toml::to_string(&wrapper).unwrap();
        assert_eq!(toml, "modes = [\"Default\", \"Combat\", \"Landing\"]\n");

        let parsed: Wrapper = toml::from_str(&toml).unwrap();
        assert_eq!(parsed, wrapper);
    }

    #[test]
    fn deserialize_rejects_non_list_value() {
        #[expect(dead_code, reason = "Only deserialized to exercise field-level errors")]
        #[derive(Debug, Deserialize)]
        struct Wrapper {
            modes: Modes,
        }

        let err = toml::from_str::<Wrapper>("modes = 42\n").unwrap_err();

        assert!(
            err.to_string()
                .contains("modes must be a flat list of strings")
        );
    }

    #[test]
    fn deserialize_rejects_non_string_entry() {
        #[expect(dead_code, reason = "Only deserialized to exercise field-level errors")]
        #[derive(Debug, Deserialize)]
        struct Wrapper {
            modes: Modes,
        }

        let err = toml::from_str::<Wrapper>("modes = [\"Default\", 42]\n").unwrap_err();

        assert!(err.to_string().contains("mode names must be strings"));
    }
}
