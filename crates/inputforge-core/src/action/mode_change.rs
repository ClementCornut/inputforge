// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

/// Strategy for changing the active input mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum ModeChangeStrategy {
    SwitchTo { mode: String },
    Temporary { mode: String },
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
    fn mode_change_strategy_temporary_serde_roundtrip() {
        let strategy = ModeChangeStrategy::Temporary {
            mode: "combat".to_owned(),
        };
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy\":\"temporary\""));
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }
}
