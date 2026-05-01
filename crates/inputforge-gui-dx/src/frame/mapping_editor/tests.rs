// Rust guideline compliant 2026-05-01

//! SSR tests for the F9 mapping editor.

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, EngineStatus};

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

// ---------------------------------------------------------------------------
// Task 13: engine-offline banner
// ---------------------------------------------------------------------------

#[test]
fn engine_offline_banner_visible_when_status_is_stopped() {
    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn h() -> Element {
        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(|| MetaSnapshot {
            engine_status: EngineStatus::Stopped,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned()],
            startup_mode: Some("Default".to_owned()),
            ..MetaSnapshot::default()
        });
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
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Engine offline"),
        "expected offline banner copy, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 14: editor header (h2 + subtitle with optional output arrow)
// ---------------------------------------------------------------------------

#[test]
fn editor_header_shows_name_as_h2() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId, OutputAddress, OutputId,
        VJoyAxis,
    };
    use std::collections::HashMap;

    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let addr = InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let actions = vec![Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        }];
        let mappings = vec![Mapping {
            input: addr.clone(),
            mode: "Default".to_owned(),
            name: Some("Yaw".to_owned()),
            actions,
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let mut state = AppState::with_profile(profile);
        state.devices.push(inputforge_core::state::DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Stick".to_owned(),
                axes: 2,
                buttons: 4,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 2],
            },
            connected: true,
        });

        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(state)),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());

        let selection: crate::frame::MappingKey = ("Default".to_owned(), addr.clone());
        let snap = ConfigSnapshot::from_state(&raw.state.read(), Some(&selection));
        let meta = use_signal(|| MetaSnapshot {
            engine_status: inputforge_core::state::EngineStatus::Running,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned()],
            startup_mode: Some("Default".to_owned()),
            current_mode: "Default".to_owned(),
            ..MetaSnapshot::default()
        });
        let config = use_signal(|| snap);
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
        view.selected_mapping
            .clone()
            .write()
            .replace(("Default".to_owned(), addr));
        use_context_provider(|| view);
        use_live_capture_provider();
        use_editor_state_provider();
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    }

    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("<h2"), "expected h2 element: {html}");
    assert!(html.contains("Yaw"), "expected mapping name: {html}");
    // Arrow present because MapToVJoy is in the action tree.
    assert!(
        html.contains('\u{2192}') || html.contains("&rarr;") || html.contains("&#8594;"),
        "expected arrow when MapToVJoy present: {html}"
    );
}

#[test]
fn editor_header_omits_output_when_no_map_to_vjoy() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId};
    use std::collections::HashMap;

    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let addr = InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let mappings = vec![Mapping {
            input: addr.clone(),
            mode: "Default".to_owned(),
            name: Some("Yaw".to_owned()),
            actions: vec![Action::Invert],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let mut state = AppState::with_profile(profile);
        state.devices.push(inputforge_core::state::DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Stick".to_owned(),
                axes: 2,
                buttons: 4,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 2],
            },
            connected: true,
        });

        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(state)),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());

        let selection: crate::frame::MappingKey = ("Default".to_owned(), addr.clone());
        let snap = ConfigSnapshot::from_state(&raw.state.read(), Some(&selection));
        let meta = use_signal(|| MetaSnapshot {
            engine_status: inputforge_core::state::EngineStatus::Running,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned()],
            startup_mode: Some("Default".to_owned()),
            current_mode: "Default".to_owned(),
            ..MetaSnapshot::default()
        });
        let config = use_signal(|| snap);
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
        view.selected_mapping
            .clone()
            .write()
            .replace(("Default".to_owned(), addr));
        use_context_provider(|| view);
        use_live_capture_provider();
        use_editor_state_provider();
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    }

    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // No arrow because no MapToVJoy.
    assert!(
        !html.contains('\u{2192}') && !html.contains("&rarr;") && !html.contains("&#8594;"),
        "expected no arrow when no MapToVJoy: {html}"
    );
}
