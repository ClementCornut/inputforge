// Rust guideline compliant 2026-05-01

//! External-edit reconciliation for the F9 mapping editor.
//!
//! Detects when `ConfigSnapshot.selected_mapping_actions` diverges from the
//! last-seen value (i.e., an external edit arrived via the polling bridge)
//! and surfaces a Warning toast per AC #27.
//!
//! The `external_edit_reset` token on [`EditorState`] is advanced on every
//! detected divergence.  Per-body `use_effect` hooks subscribe to that token
//! and reset their local `Signal`s when it advances.
//!
//! # Design note
//!
//! F9 bodies are largely stateless (they read from props and dispatch on every
//! change), so the token advance already satisfies the reset requirement for
//! this phase.  The `pending_external_reset` field on [`EditorState`] is
//! reserved for future body types that carry true local state and need to defer
//! the reset until focus, drag, and `LiveCapture` are all idle.

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::context::AppContext;
use crate::frame::mapping_editor::EditorState;
use crate::toast::{ToastLevel, ToastQueue};

/// Reconciler component mounted unconditionally inside `MappingEditor`.
///
/// On every render it compares `ConfigSnapshot.selected_mapping_actions` to
/// the last-seen value.  When a divergence is detected (and the prior value
/// was not `None`, ruling out the very first observation) it:
///
/// 1. Surfaces a [`ToastLevel::Warning`] toast (`"Mapping was edited
///    externally"`), per AC #27.
/// 2. Increments `EditorState.external_edit_reset` so that subscribed bodies
///    can reset their local state.
/// 3. Sets `EditorState.pending_external_reset` to `false` (eager reset, safe
///    for F9 where all bodies are stateless).
///
/// Renders no DOM output; the returned `Element` is always empty.
#[component]
pub(crate) fn ExternalEditReconciler() -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let toast = use_context::<ToastQueue>();

    // `last_seen` tracks the actions vector from the previous observation.
    // `Option<Option<…>>`: outer `None` = first render (baseline not yet
    // recorded); inner `None` = baseline recorded but no selection active.
    let mut last_seen: Signal<Option<Option<Vec<Action>>>> = use_signal(|| None);
    let mut external_edit_reset = editor.external_edit_reset;
    let mut pending = editor.pending_external_reset;

    use_effect(move || {
        let current: Option<Vec<Action>> = ctx.config.read().selected_mapping_actions.clone();
        let prev: Option<Option<Vec<Action>>> = last_seen.peek().clone();

        match prev {
            // First observation: record baseline without triggering a reset.
            None => {
                last_seen.set(Some(current));
            }
            // Subsequent observations: reset only when the value actually changed.
            Some(last) if last != current => {
                last_seen.set(Some(current));

                // `last` was `Some(…)` and `current` differs — an external edit
                // arrived.  Surface the toast immediately per AC #27.
                if last.is_some() {
                    toast.push(ToastLevel::Warning, "Mapping was edited externally");

                    // Advance the reset token so body subscriptions fire.
                    external_edit_reset.with_mut(|n| *n = n.wrapping_add(1));

                    // F9 bodies are stateless; eager reset is safe.
                    pending.set(false);
                }
            }
            // Value unchanged: no-op.
            Some(_) => {}
        }
    });

    rsx! {}
}
