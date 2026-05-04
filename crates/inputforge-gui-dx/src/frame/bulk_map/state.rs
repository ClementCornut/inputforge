//! Wizard state types and pure helpers.
//!
//! State machine summary: the wizard owns one source-device id, one
//! target-vjoy id, one mode picker value, an `apply_to_all_modes`
//! flag, and a `Vec<RowState>` keyed in source-input order. Each row
//! carries (a) a target override (`Option<OutputAddress>` where
//! `None` means "(do not map)") and (b) a per-row replace flag
//! defaulting to `false`. The flag is `true` only when the user has
//! explicitly clicked the row's `replace` chip.

use inputforge_core::types::{InputAddress, OutputAddress};

/// Discriminator used by the row template (kind chip + auto-map
/// algorithm). Matches the F8 mapping-list group taxonomy: Axes,
/// Buttons, Hats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowKind {
    Axis,
    Button,
    Hat,
}

/// One source input (the wizard's row identity).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RowState {
    pub kind: RowKind,
    /// Index of this input on the source device (0-based for all kinds).
    pub source_index: u8,
    /// Source address always Bound. Computed from
    /// `source_device + RowKind + source_index`.
    pub input: InputAddress,
    /// `None` means `(do not map)`; `Some` carries the user-chosen
    /// or auto-suggested target. Overflow rows default to `None`.
    pub target: Option<OutputAddress>,
    /// Sticky per-row "replace existing" flag. When `false`, a row
    /// whose `(input, mode)` already exists is skipped. When `true`,
    /// the row promotes to a replace tally and the existing mapping
    /// is overwritten.
    pub replace: bool,
}

/// Aggregate wizard state. Held by the panel component and threaded
/// to its children via signals or props as needed.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct WizardState {
    pub source_device_id: Option<inputforge_core::types::DeviceId>,
    pub target_vjoy_id: Option<u8>,
    pub mode: String,
    pub apply_to_all_modes: bool,
    pub rows: Vec<RowState>,
}

impl WizardState {
    pub(super) fn empty(default_mode: String) -> Self {
        Self {
            source_device_id: None,
            target_vjoy_id: None,
            mode: default_mode,
            apply_to_all_modes: false,
            rows: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_initial_values() {
        let s = WizardState::empty("Default".to_owned());
        assert!(s.source_device_id.is_none());
        assert!(s.target_vjoy_id.is_none());
        assert_eq!(s.mode, "Default");
        assert!(!s.apply_to_all_modes);
        assert!(s.rows.is_empty());
    }
}
