// Rust guideline compliant 2026-03-02

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

/// Strategy for changing the active input mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum ModeChangeStrategy {
    SwitchTo { mode: String },
    Temporary { mode: String },
    Previous,
    Cycle { modes: CycleModes },
}

/// A validated list of modes for cycling.
///
/// Guarantees at least 2 modes with no duplicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleModes(Vec<String>);

impl CycleModes {
    /// Create a new cycle modes list.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] if fewer than 2 modes are
    /// provided or if any mode name appears more than once.
    pub fn new(modes: Vec<String>) -> Result<Self> {
        if modes.len() < 2 {
            return Err(EngineError::InvalidConfig {
                reason: "cycle requires at least 2 modes".to_owned(),
            });
        }
        let mut seen = HashSet::new();
        for mode in &modes {
            if !seen.insert(mode.as_str()) {
                return Err(EngineError::InvalidConfig {
                    reason: format!("duplicate mode in cycle: {mode}"),
                });
            }
        }
        Ok(Self(modes))
    }

    /// Return the mode names.
    #[must_use]
    pub fn modes(&self) -> &[String] {
        &self.0
    }
}

impl Serialize for CycleModes {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CycleModes {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let modes = Vec::<String>::deserialize(deserializer)?;
        Self::new(modes).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_change_strategy_switch_to_serde_roundtrip() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "combat".to_owned(),
        };
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy\":\"switch_to\""));
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }

    #[test]
    fn mode_change_strategy_cycle_serde_roundtrip() {
        let strategy = ModeChangeStrategy::Cycle {
            modes: CycleModes::new(vec!["mode_a".to_owned(), "mode_b".to_owned()]).unwrap(),
        };
        let json = serde_json::to_string(&strategy).unwrap();
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }

    #[test]
    fn mode_change_strategy_previous_serde_roundtrip() {
        let strategy = ModeChangeStrategy::Previous;
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy\":\"previous\""));
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }

    // --- CycleModes ---

    #[test]
    fn cycle_modes_valid() {
        let modes = CycleModes::new(vec!["A".to_owned(), "B".to_owned()]).unwrap();
        assert_eq!(modes.modes(), &["A", "B"]);
    }

    #[test]
    fn cycle_modes_reject_empty() {
        let err = CycleModes::new(vec![]).unwrap_err();
        assert!(err.to_string().contains("at least 2"));
    }

    #[test]
    fn cycle_modes_reject_single() {
        let err = CycleModes::new(vec!["A".to_owned()]).unwrap_err();
        assert!(err.to_string().contains("at least 2"));
    }

    #[test]
    fn cycle_modes_reject_duplicates() {
        let err =
            CycleModes::new(vec!["A".to_owned(), "B".to_owned(), "A".to_owned()]).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn cycle_modes_serde_roundtrip() {
        let modes = CycleModes::new(vec!["X".to_owned(), "Y".to_owned(), "Z".to_owned()]).unwrap();
        let json = serde_json::to_string(&modes).unwrap();
        let back: CycleModes = serde_json::from_str(&json).unwrap();
        assert_eq!(modes, back);
    }

    #[test]
    fn cycle_modes_serde_reject_invalid() {
        let json = r#"["only_one"]"#;
        let result: std::result::Result<CycleModes, _> = serde_json::from_str(json);
        result.unwrap_err();
    }

    #[test]
    fn cycle_modes_getter() {
        let modes = CycleModes::new(vec!["A".to_owned(), "B".to_owned()]).unwrap();
        assert_eq!(modes.modes().len(), 2);
        assert_eq!(modes.modes()[0], "A");
        assert_eq!(modes.modes()[1], "B");
    }
}
