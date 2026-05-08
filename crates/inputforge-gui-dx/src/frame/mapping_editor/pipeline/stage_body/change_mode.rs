// Rust guideline compliant 2026-05-08

//! `ChangeMode` body. Renders a two-row form: strategy picker
//! (segmented Set/Hold pills) and target-mode `Select`. F14 owner.
//!
//! Hint priority (highest first):
//! 1. Empty target mode -> `"Choose a target mode"`.
//! 2. Target mode not in `MetaSnapshot.modes` -> orphan option + drift hint.
//! 3. Hold strategy with non-button primary -> selected-but-disabled Hold.
//!
//! When (2) and (3) hold simultaneously the body emits a combined hint
//! so the user can recover both errors in one edit pass.

use dioxus::prelude::*;

use inputforge_core::action::{Action, ModeChangeStrategy};

use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::StageId;

/// Hint copy. Centralised so tests can grep these strings unchanged.
pub(crate) const HINT_TARGET_EMPTY: &str = "Choose a target mode";
pub(crate) const HINT_HOLD_NOT_BUTTON: &str =
    "Hold requires a button input. Pick a button or change the strategy.";
pub(crate) const TOOLTIP_HOLD_NOT_BUTTON: &str = "Hold requires a button input.";

/// Set / Hold pill activation gate. Returns `false` when the pill is
/// `aria-disabled` or already in the active state. Both onclick handlers
/// call this; standalone-testable so acceptance #15 (Enter on aria-disabled
/// is a no-op) can be unit-verified without DOM event simulation.
#[allow(dead_code, reason = "wired in Task 13")]
pub(crate) fn pill_activates(disabled: bool, was_active: bool) -> bool {
    !disabled && !was_active
}

#[component]
pub(crate) fn ChangeModeBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    /// Current strategy (destructured from `Action::ChangeMode { strategy }`
    /// in the dispatcher).
    strategy: ModeChangeStrategy,
    root_actions: Vec<Action>,
) -> Element {
    rsx! { div { class: "if-stage__body-change-mode", "F14 body wired" } }
}
