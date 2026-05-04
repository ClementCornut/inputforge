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
use inputforge_core::types::{
    AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId, VJoyAxis, VirtualDeviceConfig,
};

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

/// Build a single-device `LiveSnapshot` with both physical axis values and
/// a single virtual-device's output axis values.
///
/// Mirrors `live_snapshot_with_axes`; adds a single `VjoyOutputValues` entry
/// at index 0 with the supplied `output_axes`. Tests pair this with
/// `add_vjoy_device` on the `AppState` so `cfg.virtual_devices[0]` aligns
/// with `live.output_values[0]`, matching the production projection in
/// `LiveSnapshot::from_state`.
fn live_snapshot_with_axes_and_outputs(
    axes: Vec<(f64, AxisPolarity)>,
    output_axes: Vec<(VJoyAxis, f64)>,
) -> LiveSnapshot {
    LiveSnapshot {
        device_inputs: vec![crate::context::DeviceInputValues {
            axes,
            buttons: vec![],
            hats: vec![],
        }],
        output_values: vec![crate::context::VjoyOutputValues {
            axes: output_axes,
            buttons: vec![],
            hats: vec![],
        }],
    }
}

/// Push a vJoy `VirtualDeviceConfig` onto `state.virtual_devices` so the
/// derived `ConfigSnapshot` includes it. Required for any OUT-row test:
/// `read_output_display` looks up by position in `cfg.virtual_devices`.
fn add_vjoy_device(state: &mut AppState, device_id: u8, axes: Vec<VJoyAxis>) {
    state.virtual_devices.push(VirtualDeviceConfig {
        device_id,
        axes,
        button_count: 0,
        hat_count: 0,
    });
}

fn axis_addr(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index },
    }
}

fn btn_addr(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index },
    }
}

fn hat_addr(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Hat { index },
    }
}

fn vjoy_x() -> inputforge_core::types::OutputAddress {
    use inputforge_core::types::{OutputAddress, OutputId};
    OutputAddress {
        device: 1,
        output: OutputId::Axis { id: VJoyAxis::X },
    }
}

fn vjoy_y() -> inputforge_core::types::OutputAddress {
    use inputforge_core::types::{OutputAddress, OutputId};
    OutputAddress {
        device: 1,
        output: OutputId::Axis { id: VJoyAxis::Y },
    }
}

fn input_index(addr: &InputAddress) -> u8 {
    match addr {
        InputAddress::Bound { input, .. } => match input {
            InputId::Axis { index } | InputId::Button { index } | InputId::Hat { index } => *index,
        },
        InputAddress::Unbound => 0,
    }
}

fn count_substring(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

fn render_with_pipeline(
    actions: &[Action],
    axes: &[(InputAddress, AxisPolarity, f64)],
    buttons: &[(InputAddress, bool)],
    hats: &[(InputAddress, inputforge_core::types::HatDirection)],
) -> String {
    render_with_pipeline_and_engine(actions, axes, buttons, hats, EngineStatus::Running)
}

fn render_with_pipeline_and_engine(
    actions: &[Action],
    axes: &[(InputAddress, AxisPolarity, f64)],
    buttons: &[(InputAddress, bool)],
    hats: &[(InputAddress, inputforge_core::types::HatDirection)],
    engine_status: EngineStatus,
) -> String {
    use inputforge_core::types::{AxisValue, InputValue};

    let axis_count = axes
        .iter()
        .map(|(addr, _, _)| usize::from(input_index(addr)) + 1)
        .max()
        .unwrap_or(1);
    let mut polarities = vec![AxisPolarity::Bipolar; axis_count];
    let mut axis_values = Vec::with_capacity(axes.len());
    for (addr, polarity, value) in axes {
        let idx = input_index(addr);
        polarities[usize::from(idx)] = *polarity;
        axis_values.push((idx, *value, *polarity));
    }

    let primary = axis_addr(0);
    let mut state =
        seeded_profile_with_polarities_and_axes(actions.to_vec(), polarities, &axis_values);
    if let Some(device) = state.devices.get_mut(0) {
        device.info.buttons = buttons
            .iter()
            .map(|(addr, _)| input_index(addr) + 1)
            .max()
            .unwrap_or(0);
        device.info.hats = hats
            .iter()
            .map(|(addr, _)| input_index(addr) + 1)
            .max()
            .unwrap_or(0);
    }
    for (addr, pressed) in buttons {
        state
            .input_cache
            .update(addr, &InputValue::Button { pressed: *pressed });
    }
    for (addr, direction) in hats {
        state.input_cache.update(
            addr,
            &InputValue::Hat {
                direction: *direction,
            },
        );
    }
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X, VJoyAxis::Y]);

    let mut live_axes = vec![(0.0, AxisPolarity::Bipolar); axis_count];
    for (addr, polarity, value) in axes {
        live_axes[usize::from(input_index(addr))] = (*value, *polarity);
    }
    let button_count = buttons
        .iter()
        .map(|(addr, _)| usize::from(input_index(addr)) + 1)
        .max()
        .unwrap_or(0);
    let mut live_buttons = vec![false; button_count];
    for (addr, pressed) in buttons {
        live_buttons[usize::from(input_index(addr))] = *pressed;
    }
    let hat_count = hats
        .iter()
        .map(|(addr, _)| usize::from(input_index(addr)) + 1)
        .max()
        .unwrap_or(0);
    let mut live_hats = vec![inputforge_core::types::HatDirection::Center; hat_count];
    for (addr, direction) in hats {
        live_hats[usize::from(input_index(addr))] = *direction;
    }
    let live = LiveSnapshot {
        device_inputs: vec![crate::context::DeviceInputValues {
            axes: live_axes,
            buttons: live_buttons,
            hats: live_hats,
        }],
        output_values: vec![crate::context::VjoyOutputValues {
            axes: vec![(VJoyAxis::X, 0.0), (VJoyAxis::Y, 0.0)],
            buttons: vec![],
            hats: vec![],
        }],
    };

    for (addr, polarity, value) in axes {
        state.input_cache.update(
            addr,
            &InputValue::Axis {
                value: AxisValue::new(*value),
                polarity: *polarity,
            },
        );
    }

    let mut vdom = harness_with_live_and_status(state, primary, live, engine_status);
    vdom.rebuild_in_place();
    render(&vdom)
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
    /// Engine status reported in the `MetaSnapshot`. Defaults to `Running`
    /// when absent so existing tests are unaffected. Overridden by the
    /// engine-stopped OUT-row tests.
    #[props(default)]
    engine_status: Option<EngineStatus>,
}

impl PartialEq for HarnessProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
            && self.addr == other.addr
            && self.current_mode == other.current_mode
            && self.engine_status == other.engine_status
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
        engine_status,
    } = props;

    let runtime_mode = current_mode.unwrap_or_else(|| "Default".to_owned());
    let runtime_engine_status = engine_status.unwrap_or(EngineStatus::Running);

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
        engine_status: runtime_engine_status,
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
            engine_status: None,
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
            engine_status: None,
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
            engine_status: None,
        },
    )
}

/// Build a `VirtualDom` with a pre-seeded `LiveSnapshot` and explicit
/// engine status. Used by OUT-row freeze tests to drive
/// `engine_status: EngineStatus::Stopped` into the harness.
fn harness_with_live_and_status(
    state: AppState,
    addr: InputAddress,
    live: LiveSnapshot,
    engine_status: EngineStatus,
) -> VirtualDom {
    VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            current_mode: None,
            initial_live: Some(Arc::new(live)),
            engine_status: Some(engine_status),
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
            engine_status: None,
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

#[test]
fn editor_live_readout_merge_layout_omits_legacy_merged_in_row() {
    use inputforge_core::types::{MergeOp, OutputAddress, OutputId};

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![
        Action::MergeAxis {
            second_input: secondary,
            operation: MergeOp::Average,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
    ];
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar, AxisPolarity::Bipolar],
        &[
            (0, 0.5, AxisPolarity::Bipolar),
            (1, -0.5, AxisPolarity::Bipolar),
        ],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
    let live = live_snapshot_with_axes_and_outputs(
        vec![(0.5, AxisPolarity::Bipolar), (-0.5, AxisPolarity::Bipolar)],
        vec![(VJoyAxis::X, 0.0)],
    );

    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);

    assert!(
        !html.contains("Merged"),
        "new analyzer-driven layout should move merge details out of the top-level IN rows; got: {html}"
    );
}

#[test]
fn editor_live_readout_composite_all_predicate_renders_two_chips_and_and_combines() {
    use inputforge_core::action::Condition;

    let actions = vec![Action::Conditional {
        condition: Condition::All {
            conditions: vec![
                Condition::ButtonPressed { input: btn_addr(0) },
                Condition::ButtonPressed { input: btn_addr(1) },
            ],
        },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![],
    }];

    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), true), (btn_addr(1), true)],
        &[],
    );
    assert_eq!(
        count_substring(
            &html,
            "if-editor__readout-chip if-editor__readout-chip--live"
        ),
        2
    );
    assert_eq!(
        count_substring(&html, super::live_readout::FROZEN_ROW_CLASS),
        0
    );

    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), true), (btn_addr(1), false)],
        &[],
    );
    assert_eq!(
        count_substring(&html, super::live_readout::FROZEN_ROW_CLASS),
        1
    );
}

#[test]
fn editor_live_readout_nested_conditional_inner_active_only_when_path_matches() {
    use inputforge_core::action::Condition;

    let inner = Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(1) },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: vec![inner],
        if_false: vec![],
    }];

    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), true), (btn_addr(1), true)],
        &[],
    );
    assert!(html.contains("X axis"));
    assert!(html.contains("Y axis"));
    assert_eq!(
        count_substring(&html, super::live_readout::FROZEN_ROW_CLASS),
        1
    );

    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), false), (btn_addr(1), true)],
        &[],
    );
    assert_eq!(
        count_substring(&html, super::live_readout::FROZEN_ROW_CLASS),
        2
    );
}

#[test]
fn editor_live_readout_engine_stopped_with_multi_out_freezes_all_rows() {
    use inputforge_core::action::Condition;

    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
    }];
    let html = render_with_pipeline_and_engine(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), true)],
        &[],
        EngineStatus::Stopped,
    );

    assert_eq!(
        count_substring(&html, "if-editor__readout-row-wrap--frozen"),
        2
    );
}

#[test]
fn editor_live_readout_hat_direction_predicate_chip_glyphs() {
    use inputforge_core::action::Condition;
    use inputforge_core::types::HatDirection;

    let actions = vec![Action::Conditional {
        condition: Condition::HatDirection {
            input: hat_addr(0),
            directions: vec![HatDirection::N, HatDirection::NE],
        },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![],
    }];
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.0)],
        &[],
        &[(hat_addr(0), HatDirection::N)],
    );

    assert!(html.contains("\u{2191}"));
    assert!(html.contains("\u{2197}"));
    assert!(html.contains("if-editor__readout-chip--live"));
}

#[test]
fn editor_live_readout_button_released_chip_suffix_and_inverted_dot() {
    use inputforge_core::action::Condition;

    let actions = vec![Action::Conditional {
        condition: Condition::ButtonReleased { input: btn_addr(0) },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![],
    }];
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.0)],
        &[(btn_addr(0), false)],
        &[],
    );
    assert!(html.contains("(released)"));
    assert!(html.contains("if-editor__readout-chip--live"));

    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.0)],
        &[(btn_addr(0), true)],
        &[],
    );
    assert!(html.contains("(released)"));
    assert!(html.contains("if-editor__readout-chip-dot--hollow"));
}

#[test]
fn editor_live_readout_per_output_polarity_disagreement() {
    use inputforge_core::action::Condition;
    use inputforge_core::types::MergeOp;

    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            },
            Action::MapToVJoy { output: vjoy_x() },
        ],
        if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
    }];
    let html = render_with_pipeline(
        &actions,
        &[
            (axis_addr(0), AxisPolarity::Unipolar, -0.5),
            (axis_addr(1), AxisPolarity::Unipolar, -0.5),
        ],
        &[(btn_addr(0), true)],
        &[],
    );
    let bipolar_count = count_substring(&html, "if-editor__readout-bar--bipolar");
    assert!(bipolar_count >= 1);
    let non_bipolar_bars = count_substring(&html, "if-editor__readout-bar") - bipolar_count;
    assert!(non_bipolar_bars >= 1);
}

#[test]
fn editor_live_readout_axis_in_range_chip_live_dot_when_in_range() {
    use inputforge_core::action::Condition;

    let actions = vec![Action::Conditional {
        condition: Condition::AxisInRange {
            input: axis_addr(1),
            min: 0.20,
            max: 0.80,
        },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![],
    }];
    let html = render_with_pipeline(
        &actions,
        &[
            (axis_addr(0), AxisPolarity::Bipolar, 0.0),
            (axis_addr(1), AxisPolarity::Bipolar, 0.5),
        ],
        &[],
        &[],
    );
    assert!(html.contains("[0.20..0.80]"));
    assert!(html.contains("if-editor__readout-chip--live"));

    let html = render_with_pipeline(
        &actions,
        &[
            (axis_addr(0), AxisPolarity::Bipolar, 0.0),
            (axis_addr(1), AxisPolarity::Bipolar, -0.3),
        ],
        &[],
        &[],
    );
    assert!(html.contains("if-editor__readout-chip-dot--hollow"));
}

/// Unipolar primary, no merge, with `MapToVJoy`. The OUT row should
/// inherit the primary's Unipolar polarity (since `find_merge_context`
/// returns None) and apply the natural-domain remap.
///
/// Primary at encoded -1 (idle pedal) goes through `MapToVJoy` unchanged;
/// the OUT row should show natural 0.00 (empty bar), not raw -1.00.
#[test]
fn editor_live_readout_unipolar_primary_no_merge_out_inherits_unipolar() {
    use inputforge_core::types::{OutputAddress, OutputId};

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
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Unipolar],
        &[(0, -1.0, AxisPolarity::Unipolar)],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
    // OUT now reads from the engine output cache (projected into
    // `live.output_values`) instead of running the pipeline. The engine
    // having written -1.0 to vJoy X for an idle Unipolar pedal is the
    // expected steady state; through `into_natural_domain` that surfaces
    // as `0.00` in the OUT row, matching the pre-change assertion.
    let live = live_snapshot_with_axes_and_outputs(
        vec![(-1.0, AxisPolarity::Unipolar)],
        vec![(VJoyAxis::X, -1.0)],
    );
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
// OUT row reads engine output cache, not the input pipeline
// ---------------------------------------------------------------------------

/// Headline regression: when the engine is stopped, OUT shows the engine
/// output cache value, NOT a value re-derived from the (still-moving)
/// input cache through the action pipeline.
#[test]
fn editor_live_readout_out_freezes_when_engine_stopped() {
    use inputforge_core::types::{OutputAddress, OutputId};

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
    // Input cache shows the user moving the stick to +0.50 right now.
    // Engine output cache is frozen at +0.20 from the last engine tick.
    // With the new wiring, OUT must read +0.20, not +0.50.
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar],
        &[(0, 0.5, AxisPolarity::Bipolar)],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
    let live = live_snapshot_with_axes_and_outputs(
        vec![(0.5, AxisPolarity::Bipolar)],
        vec![(VJoyAxis::X, 0.2)],
    );
    let mut vdom = harness_with_live_and_status(state, primary, live, EngineStatus::Stopped);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("+0.20"),
        "expected OUT to render the engine cache value `+0.20`; got: {html}"
    );
}

/// When the engine is stopped, the OUT row carries the
/// `if-editor__readout-row--frozen` modifier class so CSS can dim the bar
/// fill and percentage. The IN row does not.
#[test]
fn editor_live_readout_out_row_marks_frozen_class_when_engine_stopped() {
    use inputforge_core::types::{OutputAddress, OutputId};

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
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar],
        &[(0, 0.0, AxisPolarity::Bipolar)],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
    let live = live_snapshot_with_axes_and_outputs(
        vec![(0.0, AxisPolarity::Bipolar)],
        vec![(VJoyAxis::X, 0.0)],
    );
    let mut vdom = harness_with_live_and_status(state, primary, live, EngineStatus::Stopped);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains(super::live_readout::FROZEN_ROW_CLASS),
        "expected OUT row to carry frozen modifier class when engine stopped; got: {html}"
    );
    // The frozen class should appear exactly once (only the OUT row).
    let frozen_count = html.matches(super::live_readout::FROZEN_ROW_CLASS).count();
    assert_eq!(
        frozen_count, 1,
        "expected exactly one frozen row (OUT); found {frozen_count}; html: {html}"
    );
}

/// While the engine is running, no row carries the frozen modifier class.
#[test]
fn editor_live_readout_out_row_omits_frozen_class_when_engine_running() {
    use inputforge_core::types::{OutputAddress, OutputId};

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
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar],
        &[(0, 0.0, AxisPolarity::Bipolar)],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
    let live = live_snapshot_with_axes_and_outputs(
        vec![(0.0, AxisPolarity::Bipolar)],
        vec![(VJoyAxis::X, 0.0)],
    );
    // Default harness path uses EngineStatus::Running.
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        !html.contains(super::live_readout::FROZEN_ROW_CLASS),
        "expected no frozen rows when engine running; got: {html}"
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
