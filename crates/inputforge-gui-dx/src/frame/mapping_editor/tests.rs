// Rust guideline compliant 2026-05-01

//! SSR tests for the F9 mapping editor.

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::mode::ModeTree;
use inputforge_core::profile::Profile;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, EngineStatus};
use inputforge_core::types::{AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::frame::mapping_editor::{EditorState, MappingEditor, use_editor_state_provider};
use crate::frame::view_state::use_view_state_provider;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

// ---------------------------------------------------------------------------
// Shared test harness helpers
// ---------------------------------------------------------------------------

/// Build an `AppState` seeded with a single "Yaw" mapping on axis 0 of
/// device "dev-1" (mode "Default"), with the supplied `actions`.
///
/// The device is registered with 2 axes / 4 buttons / 0 hats so that
/// `source_label::format` can produce a human-readable subtitle.
fn seeded_profile_with_one_mapping(actions: Vec<Action>) -> AppState {
    use std::collections::HashMap;
    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let mappings = vec![Mapping {
        input: addr,
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
    state
}

/// Like [`seeded_profile_with_one_mapping`] but with explicit per-axis
/// polarities and a pre-seeded `input_cache` so pipeline evaluation
/// (`evaluate_actions_through` for merged IN / OUT rows) sees the axis
/// values that the live readout's `LiveSnapshot` will also be told
/// about. Used by Task-3 polarity-inference SSR tests.
fn seeded_profile_with_polarities_and_axes(
    actions: Vec<Action>,
    axis_polarities: Vec<AxisPolarity>,
    axis_values: &[(u8, f64, AxisPolarity)],
) -> AppState {
    use inputforge_core::types::{AxisValue, InputValue};
    use std::collections::HashMap;
    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let mappings = vec![Mapping {
        input: primary,
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
    let axes_count: u8 =
        u8::try_from(axis_polarities.len()).expect("test fixture should have <= 255 axes");
    state.devices.push(inputforge_core::state::DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "Stick".to_owned(),
            axes: axes_count,
            buttons: 0,
            hats: 0,
            instance_path: None,
            axis_polarities,
        },
        connected: true,
    });
    for &(idx, value, polarity) in axis_values {
        let addr = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: idx },
        };
        state.input_cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(value),
                polarity,
            },
        );
    }
    state
}

/// Build a single-device `LiveSnapshot` with the given axis values.
///
/// `axes` is `[(value, polarity)]` indexed by the device's axis index.
fn live_snapshot_with_axes(axes: Vec<(f64, AxisPolarity)>) -> LiveSnapshot {
    LiveSnapshot {
        device_inputs: vec![crate::context::DeviceInputValues {
            axes,
            buttons: vec![],
            hats: vec![],
        }],
        output_values: vec![],
    }
}

/// Props for the harness component.
///
/// `AppState` is not `Clone`/`PartialEq`, so it is wrapped in
/// `Arc<RwLock<_>>` at the prop boundary. `PartialEq` compares by pointer
/// equality, which is sufficient for tests since each test allocates a fresh
/// `Arc` and Dioxus only re-renders on prop change (irrelevant in SSR).
///
/// `current_mode` overrides the `MetaSnapshot::current_mode` field; when
/// `None` it defaults to `"Default"` (matching the editing mode). Task 18
/// needs `"Combat"` to drive the inactive-hint path.
///
/// `initial_live` seeds the `LiveSnapshot` signal so tests can drive
/// the live-readout component with specific axis values + polarities.
/// Defaults to the empty snapshot.
#[derive(Clone, Props)]
struct HarnessProps {
    state: Arc<RwLock<AppState>>,
    addr: InputAddress,
    /// Runtime mode reported by the engine. Defaults to `"Default"` when
    /// absent so existing tests that do not set this field are unaffected.
    #[props(default)]
    current_mode: Option<String>,
    #[props(default)]
    initial_live: Option<Arc<LiveSnapshot>>,
}

impl PartialEq for HarnessProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
            && self.addr == other.addr
            && self.current_mode == other.current_mode
            && match (&self.initial_live, &other.initial_live) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

/// Harness component: composes the full provider stack and pre-selects the
/// mapping at `props.addr` under mode "Default".
#[allow(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention"
)]
fn HarnessComponent(props: HarnessProps) -> Element {
    let HarnessProps {
        state,
        addr,
        current_mode,
        initial_live,
    } = props;

    let runtime_mode = current_mode.unwrap_or_else(|| "Default".to_owned());

    let (cmd_tx, _) = mpsc::channel();
    let raw = RawHandles {
        state,
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
    };
    use_context_provider(|| raw.clone());

    let selection: crate::frame::MappingKey = ("Default".to_owned(), addr.clone());
    let snap = ConfigSnapshot::from_state(&raw.state.read(), Some(&selection));
    let meta = use_signal(|| MetaSnapshot {
        engine_status: EngineStatus::Running,
        profile_name: Some("P".to_owned()),
        modes: vec!["Default".to_owned(), "Combat".to_owned()],
        startup_mode: Some("Default".to_owned()),
        current_mode: runtime_mode,
        ..MetaSnapshot::default()
    });
    let config = use_signal(|| snap);
    let live = use_signal(|| initial_live.as_deref().cloned().unwrap_or_default());
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

/// Build a `VirtualDom` rendering `MappingEditor` with `state` plus
/// `addr` pre-selected under mode "Default". Uses
/// `VirtualDom::new_with_props`, so no thread-local carrier is needed.
fn harness_with(state: AppState, addr: InputAddress) -> VirtualDom {
    VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            current_mode: None,
            initial_live: None,
        },
    )
}

/// Build a `VirtualDom` with a diverging runtime mode.
///
/// The editing mode stays `"Default"` (the mapping's mode); the engine
/// reports `current_mode` as something different. Used by Task 18's test.
fn harness_with_current_mode(
    state: AppState,
    addr: InputAddress,
    current_mode: &str,
) -> VirtualDom {
    VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            current_mode: Some(current_mode.to_owned()),
            initial_live: None,
        },
    )
}

/// Build a `VirtualDom` with a pre-seeded `LiveSnapshot`.
///
/// Used by live-readout merge tests to drive specific axis values and
/// polarities into the snapshot the component reads from. The snapshot
/// is wrapped in `Arc` so `HarnessProps` stays cheap-clonable.
fn harness_with_live(state: AppState, addr: InputAddress, live: LiveSnapshot) -> VirtualDom {
    VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            current_mode: None,
            initial_live: Some(Arc::new(live)),
        },
    )
}

// ---------------------------------------------------------------------------
// Basic smoke test
// ---------------------------------------------------------------------------

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
    // Compile-time gate: EditorState must expose its four signals.
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
    use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};

    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::MapToVJoy {
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    }];
    let state = seeded_profile_with_one_mapping(actions);
    let mut vdom = harness_with(state, addr);
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
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = harness_with(state, addr);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // No arrow because no MapToVJoy.
    assert!(
        !html.contains('\u{2192}') && !html.contains("&rarr;") && !html.contains("&#8594;"),
        "expected no arrow when no MapToVJoy: {html}"
    );
}

// ---------------------------------------------------------------------------
// Header inline-rename: display state hosts the F8 focus marker on the h2.
// ---------------------------------------------------------------------------

/// The header's display-mode h2 carries the F8 `data-editor-focus` marker
/// and its current name; the inline rename input only mounts after F2 /
/// right-click swaps the h2 for the editable input.
#[test]
fn editor_header_h2_carries_focus_marker_and_name() {
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = harness_with(state, addr);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("<h2"),
        "expected h2 element in display mode: {html}"
    );
    assert!(
        html.contains("data-editor-focus"),
        "h2 should carry data-editor-focus marker (F8 keyboard nav target): {html}"
    );
    assert!(
        html.contains("Yaw"),
        "h2 must render the current mapping name: {html}"
    );
    assert!(
        html.contains("tabindex=\"0\""),
        "h2 must be keyboard-focusable: {html}"
    );
    // No inline editor mounts on first paint.
    assert!(
        !html.contains("if-editor__title-input"),
        "title input must not be present in display mode: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 19: undo recap footer
// ---------------------------------------------------------------------------

#[test]
fn editor_undo_recap_shows_label_and_kbd_hint() {
    use inputforge_core::action::Mapping as CoreMapping;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    use crate::frame::mapping_editor::undo_log::UndoKind;

    // Harness component that seeds one undo entry before rendering.
    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn UndoHarness(props: HarnessProps) -> Element {
        let HarnessProps { state, addr, .. } = props;

        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state,
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());

        let selection: crate::frame::MappingKey = ("Default".to_owned(), addr.clone());
        let snap = ConfigSnapshot::from_state(&raw.state.read(), Some(&selection));
        let meta = use_signal(|| MetaSnapshot {
            engine_status: EngineStatus::Running,
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
            .replace(("Default".to_owned(), addr.clone()));
        use_context_provider(|| view);
        use_live_capture_provider();

        // Install EditorState and immediately seed one undo entry so that
        // UndoRecap has a label to display.
        let mut editor = use_editor_state_provider();
        let mapping_key: crate::frame::MappingKey = ("Default".to_owned(), addr.clone());
        let before = CoreMapping {
            input: addr,
            mode: "Default".to_owned(),
            name: Some("X".to_owned()),
            actions: vec![],
        };
        editor.undo_log.write().push_edit(
            mapping_key,
            before,
            UndoKind::Rename,
            "rename: 'X' -> 'Yaw'".to_owned(),
        );

        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    }

    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = VirtualDom::new_with_props(
        UndoHarness,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            current_mode: None,
            initial_live: None,
        },
    );
    vdom.rebuild_in_place();
    let html = render(&vdom);

    assert!(
        html.contains("rename:"),
        "expected undo label in footer; got: {html}"
    );
    // U+2303 CONTROL (the up-caret glyph used for Ctrl) followed by Z.
    assert!(
        html.contains('\u{2303}'),
        "expected control-glyph (U+2303) in kbd hint; got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 16: input field with rebind action arming LiveCapture
// ---------------------------------------------------------------------------

#[test]
fn editor_input_field_renders_source_label_and_rebind_button() {
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = harness_with(state, addr);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Stick"),
        "expected source device label; got: {html}"
    );
    assert!(
        html.contains("rebind"),
        "expected rebind button; got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 17: live readout (IN/OUT bars + merge layout)
// ---------------------------------------------------------------------------

#[test]
fn editor_live_readout_renders_in_row() {
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = harness_with(state, addr);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-editor__readout-label"),
        "expected readout label cell; got: {html}"
    );
    assert!(
        html.contains(">IN<") || html.contains(">IN "),
        "IN row label; got: {html}"
    );
}

#[test]
fn editor_live_readout_omits_out_when_no_map_to_vjoy() {
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = harness_with(state, addr);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(!html.contains(">OUT<"), "OUT row must be hidden: {html}");
}

#[test]
fn editor_live_readout_renders_out_when_map_to_vjoy_present() {
    use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};

    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::MapToVJoy {
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    }];
    let state = seeded_profile_with_one_mapping(actions);
    let mut vdom = harness_with(state, addr);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("OUT"),
        "OUT row should render with MapToVJoy: {html}"
    );
}

// ---------------------------------------------------------------------------
// F9 follow-up: live readout polarity inference for merge results
// ---------------------------------------------------------------------------

/// Rudder-pedals scenario: two unipolar pedals merged via Bidirectional.
/// Both at idle (encoded -1, -1); diff = 0 (centered bipolar).
/// Expected: IN row formats `+0.00` (bipolar with sign), bar centered.
#[test]
fn editor_live_readout_bidirectional_uu_idle_renders_centered_bipolar_in() {
    use inputforge_core::types::MergeOp;

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![Action::MergeAxis {
        second_input: secondary,
        operation: MergeOp::Bidirectional,
    }];
    let state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Unipolar, AxisPolarity::Unipolar],
        &[
            (0, -1.0, AxisPolarity::Unipolar),
            (1, -1.0, AxisPolarity::Unipolar),
        ],
    );
    let live = live_snapshot_with_axes(vec![
        (-1.0, AxisPolarity::Unipolar),
        (-1.0, AxisPolarity::Unipolar),
    ]);
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // The merged IN row inherits Bidirectional's Bipolar output polarity,
    // so the format includes a sign prefix and reads exactly `+0.00`.
    assert!(
        html.contains("+0.00"),
        "expected merged IN to render +0.00 for UU idle Bidirectional; got: {html}"
    );
    // IN 2 row inherits Unipolar; format omits the sign and reads `0.00`.
    assert!(
        html.contains(">IN 2<") || html.contains(">IN 2 "),
        "expected IN 2 row label; got: {html}"
    );
    // Bipolar bars at center: width 0%, anchored at the 50% midline.
    // The merged-IN row at idle should hit this exact style.
    assert!(
        html.contains("left: 50%; right: auto; width: 0%"),
        "expected merged IN bar centered (0% width, 50% anchor); got: {html}"
    );
}

/// Rudder UU Bidirectional, one pedal half-pressed: the user-reported
/// bug case. Encoded inputs (0, -1); natural (0.5, 0); diff = 0.5.
/// Merged IN row inherits Bipolar polarity and reads `+0.50`. Bar
/// grows rightward to half its half-width: `width: 25%`, anchored at
/// the 50% midline.
///
/// Pre-fix the encoded subtraction returned 1.0, rendering `+1.00`
/// (full bar) when only one pedal was at half-press.
#[test]
fn editor_live_readout_bidirectional_uu_half_press_renders_half_deflection() {
    use inputforge_core::types::MergeOp;

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![Action::MergeAxis {
        second_input: secondary,
        operation: MergeOp::Bidirectional,
    }];
    let state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Unipolar, AxisPolarity::Unipolar],
        &[
            (0, 0.0, AxisPolarity::Unipolar),
            (1, -1.0, AxisPolarity::Unipolar),
        ],
    );
    let live = live_snapshot_with_axes(vec![
        (0.0, AxisPolarity::Unipolar),
        (-1.0, AxisPolarity::Unipolar),
    ]);
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // Merged IN reads bipolar +0.50, NOT +1.00.
    assert!(
        html.contains("+0.50"),
        "expected merged IN +0.50 for half-press Bidirectional; got: {html}"
    );
    assert!(
        !html.contains("+1.00"),
        "merged IN must not render +1.00 for half-press; got: {html}"
    );
    // Bipolar bar grows from 50% midline rightward; visual maximum is
    // half the container, so 50% deflection -> width: 25%.
    assert!(
        html.contains("left: 50%; right: auto; width: 25%"),
        "expected merged IN bar at half-deflection (width: 25%, 50% anchor); got: {html}"
    );
}

/// Average of two unipolar pedals at idle (encoded -1, -1).
/// Expected: IN row inherits Unipolar polarity (per truth table); the
/// natural-domain remap turns encoded -1 into displayed 0.00 with no
/// sign prefix, and the bar is empty (anchored at left, zero width).
#[test]
fn editor_live_readout_average_uu_idle_renders_empty_unipolar_in() {
    use inputforge_core::types::MergeOp;

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![Action::MergeAxis {
        second_input: secondary,
        operation: MergeOp::Average,
    }];
    let state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Unipolar, AxisPolarity::Unipolar],
        &[
            (0, -1.0, AxisPolarity::Unipolar),
            (1, -1.0, AxisPolarity::Unipolar),
        ],
    );
    let live = live_snapshot_with_axes(vec![
        (-1.0, AxisPolarity::Unipolar),
        (-1.0, AxisPolarity::Unipolar),
    ]);
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // Unipolar format omits the sign. `0.00` (no leading +) appears for
    // both per-input rows AND the merged IN row.
    assert!(
        !html.contains("+0.00"),
        "no bipolar `+0.00` should appear for Average UU idle; got: {html}"
    );
    assert!(
        html.contains("0.00"),
        "expected unipolar `0.00` somewhere in the readout; got: {html}"
    );
    // The merged IN row's bar has `width: 0%` (empty), grown from the
    // left edge.
    assert!(
        html.contains("left: 0; right: auto; width: 0%"),
        "expected empty unipolar IN bar; got: {html}"
    );
}

/// Average of two unipolar pedals fully pressed (encoded 1, 1).
/// Expected: IN row Unipolar with natural value 1.0, format `1.00`,
/// bar at full width.
#[test]
fn editor_live_readout_average_uu_full_press_renders_full_unipolar_in() {
    use inputforge_core::types::MergeOp;

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![Action::MergeAxis {
        second_input: secondary,
        operation: MergeOp::Average,
    }];
    let state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Unipolar, AxisPolarity::Unipolar],
        &[
            (0, 1.0, AxisPolarity::Unipolar),
            (1, 1.0, AxisPolarity::Unipolar),
        ],
    );
    let live = live_snapshot_with_axes(vec![
        (1.0, AxisPolarity::Unipolar),
        (1.0, AxisPolarity::Unipolar),
    ]);
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("1.00"),
        "expected `1.00` (Unipolar full press); got: {html}"
    );
    assert!(
        html.contains("left: 0; right: auto; width: 100%"),
        "expected full unipolar IN bar; got: {html}"
    );
}

/// Bipolar+Bipolar Average regression: behavior must not change when
/// both inputs are Bipolar. Anchors the "no regression" promise from
/// the plan's acceptance criteria.
#[test]
fn editor_live_readout_average_bb_renders_bipolar_unchanged() {
    use inputforge_core::types::MergeOp;

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![Action::MergeAxis {
        second_input: secondary,
        operation: MergeOp::Average,
    }];
    // Primary at +0.5, secondary at -0.5. Average = 0.0 (bipolar center).
    let state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar, AxisPolarity::Bipolar],
        &[
            (0, 0.5, AxisPolarity::Bipolar),
            (1, -0.5, AxisPolarity::Bipolar),
        ],
    );
    let live = live_snapshot_with_axes(vec![
        (0.5, AxisPolarity::Bipolar),
        (-0.5, AxisPolarity::Bipolar),
    ]);
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // Merged IN should be Bipolar (sign prefix) and read `+0.00`.
    assert!(
        html.contains("+0.00"),
        "expected bipolar `+0.00` for BB Average regression; got: {html}"
    );
    // Per-input rows show `+0.50` and `-0.50` (Bipolar formatting).
    assert!(
        html.contains("+0.50"),
        "expected primary `+0.50` per-input; got: {html}"
    );
    assert!(
        html.contains("-0.50"),
        "expected secondary `-0.50` per-input; got: {html}"
    );
}

/// Unipolar primary, no merge, with `MapToVJoy`. The OUT row should
/// inherit the primary's Unipolar polarity (since `find_merge_context`
/// returns None) and apply the natural-domain remap.
///
/// Primary at encoded -1 (idle pedal) goes through `MapToVJoy` unchanged;
/// the OUT row should show natural 0.00 (empty bar), not raw -1.00.
#[test]
fn editor_live_readout_unipolar_primary_no_merge_out_inherits_unipolar() {
    use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::MapToVJoy {
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    }];
    let state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Unipolar],
        &[(0, -1.0, AxisPolarity::Unipolar)],
    );
    let live = live_snapshot_with_axes(vec![(-1.0, AxisPolarity::Unipolar)]);
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains(">OUT<") || html.contains(">OUT "),
        "expected OUT row to render; got: {html}"
    );
    // Encoded -1 -> natural 0 via into_natural_domain. Format `0.00`
    // (no sign prefix because Unipolar). The pre-Task-3 bug rendered
    // this as `-1.00` (encoded passthrough).
    assert!(
        !html.contains("-1.00"),
        "Unipolar OUT must NOT render encoded `-1.00`; got: {html}"
    );
    assert!(
        html.contains("0.00"),
        "expected natural-domain `0.00` for Unipolar OUT idle; got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 18: inactive-runtime hint banner
// ---------------------------------------------------------------------------

/// When the engine is Running but its current mode differs from the mapping's
/// editing mode, `InactiveHint` must render the mode-mismatch copy.
///
/// Harness: engine Running, `current_mode = "Combat"`, mapping in mode `"Default"`.
#[test]
fn editor_inactive_hint_visible_when_modes_diverge() {
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = harness_with_current_mode(state, addr, "Combat");
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Engine is in"),
        "expected inactive-hint prefix copy; got: {html}"
    );
    assert!(
        html.contains("Mapping fires only in"),
        "expected inactive-hint suffix copy; got: {html}"
    );
}
