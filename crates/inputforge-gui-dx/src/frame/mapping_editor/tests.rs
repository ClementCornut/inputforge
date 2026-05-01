// Rust guideline compliant 2026-05-01

//! SSR tests for the F9 mapping editor.

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::frame::mapping_editor::{EditorState, MappingEditor, use_editor_state_provider};
use crate::frame::view_state::use_view_state_provider;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

/// Compose all required providers and render `MappingEditor` in SSR.
///
/// `ViewState.selected_mapping` starts as `None` (default), so the empty
/// state branch is taken on first render.
fn harness() -> Element {
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let raw = RawHandles {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
    };
    use_context_provider(|| raw.clone());

    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let ctx = AppContext {
        state: Arc::clone(&raw.state),
        commands: raw.commands.clone(),
        settings: Arc::clone(&raw.settings),
        meta,
        config,
        live,
    };
    use_context_provider(|| ctx);

    let view = use_view_state_provider(meta);
    use_context_provider(|| view);
    use_live_capture_provider();
    use_editor_state_provider();
    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });

    rsx! { MappingEditor {} }
}

#[test]
fn editor_renders_empty_state_when_no_selection() {
    let mut vdom = VirtualDom::new(harness);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Select a mapping"),
        "expected empty state title, got: {html}"
    );
    assert!(html.contains("if-editor"));
}

// ---------------------------------------------------------------------------
// Legacy tests migrated from the former inline `mod tests` block.
// ---------------------------------------------------------------------------

#[test]
fn editor_state_field_types_compile() {
    // Compile-time gate: EditorState must expose all five signals.
    use std::collections::{HashMap, HashSet};

    use crate::frame::mapping_editor::{
        StageMenuState,
        undo_log::{StageId, UndoLog},
    };

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

    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn Child() -> Element {
        let _live = use_context::<crate::patterns::live_capture::LiveCapture>();
        let editor = use_context::<EditorState>();
        // Touch every field so a missing one causes a compile error.
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

        use_live_capture_provider();
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
