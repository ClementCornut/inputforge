// Rust guideline compliant 2026-05-01

//! Last committed change label plus Ctrl+Z keyboard hint.
//!
//! Reads `EditorState.undo_log.last_label(mapping_key)` for the active
//! selection. Renders nothing when the undo stack is empty for that key.

use dioxus::prelude::*;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;

/// Footer strip that recalls the last committed change and prompts the user
/// to undo it via the Ctrl+Z shortcut.
///
/// # Visibility contract
///
/// Returns an empty element when no undo entry exists for `mapping_key` so
/// the layout does not reserve vertical space for an empty footer row.
///
/// # Unicode glyphs
///
/// - U+2303 UP ARROWHEAD (`⌃`) represents the Control modifier key, matching
///   the macOS convention displayed throughout the spec.
/// - U+00B7 MIDDLE DOT (`·`) is the separator between the label and the hint,
///   consistent with other subtitle rows in the mapping editor.
#[component]
pub(crate) fn UndoRecap(mapping_key: MappingKey) -> Element {
    let editor = use_context::<EditorState>();
    let log = editor.undo_log.read();
    let label = log.last_label(&mapping_key);

    let Some(label) = label else {
        return rsx! {};
    };

    rsx! {
        div { class: "if-editor__footer",
            span { class: "if-editor__footer-label", "{label}" }
            // U+00B7 MIDDLE DOT separates the label from the kbd hint.
            span { class: "if-editor__footer-sep", " \u{00b7} " }
            // U+2303 UP ARROWHEAD represents the Control modifier key.
            kbd { class: "if-editor__kbd", "\u{2303}Z" }
            span { class: "if-editor__footer-sep", " to undo" }
        }
    }
}
