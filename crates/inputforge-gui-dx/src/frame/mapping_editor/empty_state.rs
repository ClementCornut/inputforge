// Rust guideline compliant 2026-05-01

//! "Select a mapping" empty-state placeholder rendered when no mapping row is
//! selected in the rail.

use dioxus::prelude::*;

/// Centered call-to-action shown inside `if-editor` when
/// `ViewState.selected_mapping` is `None`.
#[component]
pub(crate) fn EmptyState() -> Element {
    rsx! {
        div { class: "if-editor__empty",
            div { class: "if-editor__empty-title", "Select a mapping" }
            div { class: "if-editor__empty-helper",
                "Pick a row in the rail, or click "
                kbd { class: "if-editor__kbd", "+ Add mapping" }
                " below the list to start one."
            }
        }
    }
}
