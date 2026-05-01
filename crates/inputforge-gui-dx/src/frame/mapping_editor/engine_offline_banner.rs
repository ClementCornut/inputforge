// Rust guideline compliant 2026-05-01

//! Sticky engine-offline banner shown above the editor frame.
//!
//! Renders only when `MetaSnapshot::engine_status` is [`EngineStatus::Stopped`].
//! When visible it surfaces "Engine offline. Edits not applied." and a ghost
//! `Restart engine` button that dispatches [`EngineCommand::Activate`].
//!
//! # Banner precedence
//!
//! Per the F9 spec (choice 20) this banner has higher precedence than the
//! inactive-runtime hint (Task 18): when this banner is visible the Task 18
//! hint is suppressed.  This component has no awareness of Task 18; Task 18
//! reads `engine_status` itself and self-suppresses.

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::state::EngineStatus;

use crate::context::AppContext;

/// Sticky banner indicating that the engine is offline.
///
/// Renders an empty element when the engine is running or paused.
/// Renders the offline notice and a restart button when the engine is stopped.
#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub(crate) fn EngineOfflineBanner() -> Element {
    let ctx = use_context::<AppContext>();
    let status = ctx.meta.read().engine_status;

    // Only Stopped maps to "offline"; Running and Paused are online states.
    // EngineStatus has no Crashed variant in this codebase (verified 2026-05-01).
    if status != EngineStatus::Stopped {
        return rsx! {};
    }

    let cmd_tx = ctx.commands.clone();
    let on_restart = move |_| {
        // Ignore send errors: the engine thread may have already exited.
        let _ = cmd_tx.send(EngineCommand::Activate);
        tracing::info!(
            target: "f9::mapping_editor",
            action = "restart_engine",
            "user requested engine restart from offline banner",
        );
    };

    rsx! {
        div {
            class: "if-editor__offline-banner",
            role: "status",
            "aria-live": "polite",
            div { class: "if-editor__offline-text", "Engine offline. Edits not applied." }
            button {
                r#type: "button",
                class: "if-editor__offline-action",
                onclick: on_restart,
                "Restart engine"
            }
        }
    }
}
