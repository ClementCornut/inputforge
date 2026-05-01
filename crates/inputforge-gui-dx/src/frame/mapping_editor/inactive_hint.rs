// Rust guideline compliant 2026-05-01

//! Inactive-runtime hint banner. See spec choice 8 + 9.
//!
//! Visible only when the engine is `Running` AND the engine's current runtime
//! mode differs from the mapping's editing mode. When the engine is offline
//! the `EngineOfflineBanner` (Task 13) takes precedence and this component
//! self-suppresses.

use dioxus::prelude::*;

use inputforge_core::state::EngineStatus;

use crate::context::AppContext;
use crate::frame::view_state::ViewState;

/// Tinted card shown when the mapping's editing mode does not match the
/// engine's current runtime mode.
///
/// # Banner precedence
///
/// Per the F9 spec (Task 13's rule): engine-offline subsumes mode-mismatch.
/// This component reads `engine_status` itself and returns an empty element
/// when the engine is not `Running`, so the offline banner is always the
/// dominant signal.
#[component]
pub(crate) fn InactiveHint() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    let engine_status = ctx.meta.read().engine_status;
    let runtime = ctx.meta.read().current_mode.clone();
    let editing = view.editing_mode.read().clone();

    // Precedence: engine-offline subsumes mode-mismatch.
    if !matches!(engine_status, EngineStatus::Running) {
        return rsx! {};
    }

    // No hint when modes agree or runtime is empty (no profile loaded).
    if runtime == editing || runtime.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "if-editor__inactive-hint",
            role: "status",
            "aria-live": "polite",
            "Engine is in "
            strong { "{runtime}" }
            ". Mapping fires only in "
            strong { "{editing}" }
            "."
        }
    }
}
