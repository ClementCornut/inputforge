// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

/// A keyboard key combination (key + modifiers).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyCombo {
    pub key: String,
    pub modifiers: Vec<KeyModifier>,
}

/// Keyboard modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyModifier {
    Ctrl,
    Shift,
    Alt,
    Win,
}

/// Axis merge operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeOp {
    Bidirectional,
    Average,
    Maximum,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_combo_serde_roundtrip() {
        let combo = KeyCombo {
            key: "F1".to_owned(),
            modifiers: vec![KeyModifier::Ctrl, KeyModifier::Shift],
        };
        let json = serde_json::to_string(&combo).unwrap();
        let back: KeyCombo = serde_json::from_str(&json).unwrap();
        assert_eq!(combo, back);
    }

    #[test]
    fn merge_op_all_variants() {
        let ops = [MergeOp::Bidirectional, MergeOp::Average, MergeOp::Maximum];
        assert_eq!(ops.len(), 3);
    }
}
