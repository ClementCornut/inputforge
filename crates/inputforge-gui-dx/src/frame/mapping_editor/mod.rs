// Rust guideline compliant 2026-05-01

//! F9 mapping editor (center column). See
//! `docs/superpowers/specs/2026-04-30-f9-mapping-editor-design.md`.

#![allow(
    dead_code,
    reason = "Sub-modules expose APIs that orchestrator + Tasks 12+ consume; \
              clippy's reachability check loses some pub(crate) items here."
)]

pub(crate) mod pipeline;
pub(crate) mod undo_log;

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use crate::frame::mapping_editor::undo_log::{StageId, UndoLog};

/// Right-click stage menu state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StageMenuState {
    pub stage: StageId,
    /// page-space anchor coordinates
    pub x: f64,
    pub y: f64,
}

/// Editor-internal context, parallel to `LiveCapture` and `ToastQueue`.
///
/// Installed once via `use_editor_state_provider` from `app_root`.
/// Components read via `use_context::<EditorState>()`.
#[derive(Clone, Copy)]
pub(crate) struct EditorState {
    /// Per-mapping undo stacks. Cleared on profile flip via Task 32's
    /// `DirtyConfirmDialog::onsave` callback.
    pub undo_log: Signal<UndoLog>,
    /// Stage IDs that are currently expanded. Resets on selection change
    /// AND on every structural mutation (insert/remove) — see Task 11.
    pub expanded_stages: Signal<HashSet<StageId>>,
    /// Right-click menu state (anchor + target stage).
    pub stage_menu: Signal<Option<StageMenuState>>,
    /// Per-stage validation hints surfaced in the stage header summary
    /// slot per spec lines 587-589. Bodies write on render; the stage
    /// header reads. Cleared on every structural mutation — see Task 11.
    pub malformed_hints: Signal<HashMap<StageId, String>>,
    /// External-edit reconciliation token. Incremented by the polling
    /// task (bridge.rs) on every external snapshot change. Bodies
    /// subscribe via `use_effect` and reset their local Signals when the
    /// token advances. See Task 33.
    pub external_edit_reset: Signal<u64>,
}

/// Allocate signals and install `EditorState` in context. Call exactly
/// once from `app_root`, the provider self-installs.
pub(crate) fn use_editor_state_provider() -> EditorState {
    let undo_log: Signal<UndoLog> = use_signal(UndoLog::default);
    let expanded_stages: Signal<HashSet<StageId>> = use_signal(HashSet::new);
    let stage_menu: Signal<Option<StageMenuState>> = use_signal(|| None);
    let malformed_hints: Signal<HashMap<StageId, String>> = use_signal(HashMap::new);
    let external_edit_reset: Signal<u64> = use_signal(|| 0_u64);

    let state = EditorState {
        undo_log,
        expanded_stages,
        stage_menu,
        malformed_hints,
        external_edit_reset,
    };
    use_context_provider(|| state);
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_state_field_types_compile() {
        // Compile-time gate: EditorState must expose all five signals.
        fn _assert(state: EditorState) {
            let _: Signal<UndoLog> = state.undo_log;
            let _: Signal<HashSet<StageId>> = state.expanded_stages;
            let _: Signal<Option<StageMenuState>> = state.stage_menu;
            let _: Signal<HashMap<StageId, String>> = state.malformed_hints;
            let _: Signal<u64> = state.external_edit_reset;
        }
    }

    #[test]
    fn editor_state_provider_mounts_and_reads_via_use_context() {
        // SSR smoke test: provider installs both LiveCapture and EditorState;
        // a child renders and reads both via `use_context`.
        use std::sync::{Arc, mpsc};

        use dioxus::prelude::*;
        use dioxus_ssr::render;
        use parking_lot::RwLock;

        use inputforge_core::settings::AppSettings;
        use inputforge_core::state::AppState;

        use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};

        #[allow(
            non_snake_case,
            reason = "Dioxus components are PascalCase by convention"
        )]
        fn Child() -> Element {
            let _live = use_context::<crate::patterns::live_capture::LiveCapture>();
            let editor = use_context::<EditorState>();
            // Touch every field so a missing one would cause a compile error.
            let undo_log = editor.undo_log.read();
            assert_eq!(undo_log.stacks.len(), 0, "fresh undo_log must be empty");
            assert_eq!(
                *editor.external_edit_reset.read(),
                0_u64,
                "external_edit_reset must start at 0"
            );
            rsx! { div { "ok" } }
        }

        #[allow(
            non_snake_case,
            reason = "Dioxus components are PascalCase by convention"
        )]
        fn Root() -> Element {
            // Provide AppContext stub that use_live_capture_provider requires.
            let (cmd_tx, _cmd_rx) = mpsc::channel();
            let ctx = AppContext {
                state: Arc::new(RwLock::new(AppState::new())),
                commands: cmd_tx,
                settings: Arc::new(AppSettings::default()),
                meta: use_signal(MetaSnapshot::default),
                config: use_signal(ConfigSnapshot::default),
                live: use_signal(LiveSnapshot::default),
            };
            use_context_provider(|| ctx);

            crate::patterns::live_capture::use_live_capture_provider();
            use_editor_state_provider();
            rsx! { Child {} }
        }

        let mut vdom = VirtualDom::new(Root);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("ok"),
            "child must render with both contexts available; got: {html}"
        );
    }
}
