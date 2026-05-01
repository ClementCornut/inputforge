// Rust guideline compliant 2026-05-01

//! Deferred stage bodies for F10 (`ResponseCurve`), F11 (Deadzone), F14 (`ChangeMode`).
//! All three render the same single-string spec caption per spec line 300.
//!
//! When F10/F11/F14 ship, they replace ONE component in this module without
//! touching the `StageBody` dispatcher, the `StageHeader` API, or the `EditorState`
//! provider. See the F9->F10/F11/F14 sequencing constraint in the plan header.

use dioxus::prelude::*;

const PLACEHOLDER_CAPTION: &str = "F10 / F11 / F14 owns this body";

#[component]
pub(crate) fn ResponseCurvePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}

#[component]
pub(crate) fn DeadzonePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}

#[component]
pub(crate) fn ChangeModePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}
