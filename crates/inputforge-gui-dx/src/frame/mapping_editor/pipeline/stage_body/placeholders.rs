// Rust guideline compliant 2026-05-03

//! Deferred stage body for F14 (`ChangeMode`). Renders a single-string spec
//! caption per spec line 300.
//!
//! F10 (`ResponseCurve`) shipped in Task 16 and F11 (`Deadzone`) shipped in
//! Task 15 of the F11 plan; neither uses a placeholder anymore. When F14
//! ships, it replaces the remaining component in this module without
//! touching the `StageBody` dispatcher, the `StageHeader` API, or the
//! `EditorState` provider. See the F9->F14 sequencing constraint in the
//! plan header.

use dioxus::prelude::*;

const PLACEHOLDER_CAPTION: &str = "F14 owns this body";

#[component]
pub(crate) fn ChangeModePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}
