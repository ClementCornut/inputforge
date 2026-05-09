//! Prune-confirm dialog: thin wrapper around `DestructiveConfirmDialog`.
//!
//! Renders the F15 prune-specific copy:
//!   "Reduce snapshot buffer to N? K unpinned snapshots will be deleted
//!    from <profile>. Pinned snapshots are kept."

// Rust guideline compliant 2026-05-09

use dioxus::prelude::*;

use crate::patterns::DestructiveConfirmDialog;

#[component]
pub(crate) fn PruneConfirmDialog(
    open: Signal<bool>,
    candidate_max: usize,
    will_remove: usize,
    profile_name: String,
    oncancel: EventHandler<()>,
    onconfirm: EventHandler<()>,
) -> Element {
    let title = format!("Reduce snapshot buffer to {candidate_max}?");
    // `DialogDescription` already wraps its children in a `<p>`; passing
    // bare text here avoids nested `<p>` (Task 7 review fold-in).
    let body = format!(
        "{will_remove} unpinned snapshots will be deleted from {profile_name}. \
         Pinned snapshots are kept."
    );

    rsx! {
        DestructiveConfirmDialog {
            open: open,
            title: Some(title),
            description: rsx! { "{body}" },
            confirm_label: "Reduce".to_owned(),
            oncancel: oncancel,
            onconfirm: onconfirm,
        }
    }
}
