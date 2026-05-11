// Rust guideline compliant 2026-05-10

mod state;

pub use state::ModeState;

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

/// Ordered flat list of profile modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Modes(Vec<String>);

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

    /// Always `false` because [`Modes`] is non-empty by construction.
    ///
    /// Defined so the type satisfies clippy's
    /// [`len_without_is_empty`](https://rust-lang.github.io/rust-clippy/master/index.html#len_without_is_empty)
    /// lint and so callers that hold the type behind a generic length
    /// trait do not silently get the wrong answer.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|candidate| candidate == name)
    }

    /// Return a new mode list with `name` appended at the tail.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] if `name` collides with an
    /// existing mode under ASCII case folding.
    pub fn with_appended(&self, name: &str) -> Result<Self> {
        let mut names = self.0.clone();
        names.push(name.to_owned());
        Self::new(names)
    }

    /// Return a new mode list with the mode named `from` renamed to `to`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ModeNotFound`] if `from` is not in the list,
    /// or [`EngineError::InvalidConfig`] if `to` collides with another
    /// existing mode under ASCII case folding.
    pub fn with_renamed(&self, from: &str, to: &str) -> Result<Self> {
        let Some(index) = self.0.iter().position(|name| name == from) else {
            return Err(EngineError::ModeNotFound {
                name: from.to_owned(),
            });
        };

        let mut names = self.0.clone();
        to.clone_into(&mut names[index]);
        Self::new(names)
    }

    /// Return a new mode list with the mode named `name` removed.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::ModeNotFound`] if `name` is not in the list,
    /// or [`EngineError::InvalidConfig`] if removing it would leave the
    /// list empty.
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
