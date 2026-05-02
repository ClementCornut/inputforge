// Rust guideline compliant 2026-05-02

//! Deferred stage bodies for F11 (Deadzone) and F14 (`ChangeMode`).
//! Both render the same single-string spec caption per spec line 300.
//!
//! F10 (`ResponseCurve`) shipped in Task 16 and no longer uses a placeholder.
//! When F11/F14 ship, they replace ONE component in this module without
//! touching the `StageBody` dispatcher, the `StageHeader` API, or the
//! `EditorState` provider. See the F9->F11/F14 sequencing constraint in the
//! plan header.

use dioxus::prelude::*;

const PLACEHOLDER_CAPTION: &str = "F10 / F11 / F14 owns this body";

#[component]
pub(crate) fn DeadzonePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}

#[component]
pub(crate) fn ChangeModePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}
