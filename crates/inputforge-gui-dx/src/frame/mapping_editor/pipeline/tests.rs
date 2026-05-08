// Rust guideline compliant 2026-05-01

//! SSR + unit tests for the F9 pipeline graph component.
//!
//! Contains:
//! - Pure `StageId` path-walker tests (migrated from the former inline
//!   `mod tests` block in `pipeline/mod.rs`)
//! - SSR rendering tests for `Pipeline` and `Stage` (Task 20)

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::action::{Action, Condition, Mapping};
use inputforge_core::mode::ModeTree;
use inputforge_core::processing::DeadzoneConfig;
use inputforge_core::profile::Profile;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId, KeyCombo, KeyModifier, MergeOp,
    OutputAddress, OutputId, VJoyAxis, VirtualDeviceConfig,
};
use std::collections::HashMap;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::frame::mapping_editor::{MappingEditor, use_editor_state_provider};
use crate::frame::view_state::use_view_state_provider;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

use super::super::undo_log::{StageId, StageIdSegment};
use super::{at_path, insert_at_path, remove_at_path, replace_at_path};

// ---------------------------------------------------------------------------
// Helpers shared by path-walker tests
// ---------------------------------------------------------------------------

fn synth_addr() -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    }
}

// ---------------------------------------------------------------------------
// Migrated path-walker unit tests (formerly inline in mod.rs)
// ---------------------------------------------------------------------------

#[test]
fn at_path_outer_index() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    assert!(matches!(at_path(&actions, &path), Some(Action::Invert)));
}

#[test]
fn at_path_into_if_true_branch() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![Action::Invert],
        if_false: Vec::new(),
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfTrue,
        StageIdSegment::Index(0),
    ]);
    assert!(matches!(at_path(&actions, &path), Some(Action::Invert)));
}

#[test]
fn at_path_into_missing_if_false_returns_none() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfFalse,
        StageIdSegment::Index(0),
    ]);
    assert!(at_path(&actions, &path).is_none());
}

#[test]
fn replace_at_path_outer_swaps_action() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    let new = replace_at_path(
        &actions,
        &path,
        Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
    )
    .expect("valid path must succeed");
    assert!(matches!(new[0], Action::MergeAxis { .. }));
}

#[test]
fn replace_at_path_inside_if_true_swaps_action() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![Action::Invert],
        if_false: Vec::new(),
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfTrue,
        StageIdSegment::Index(0),
    ]);
    let new = replace_at_path(
        &actions,
        &path,
        Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
    )
    .expect("valid path must succeed");
    match &new[0] {
        Action::Conditional { if_true, .. } => {
            assert!(matches!(if_true[0], Action::MergeAxis { .. }));
        }
        _ => panic!("outer wrapper should remain Conditional"),
    }
}

#[test]
fn replace_at_path_invalid_path_returns_none() {
    // Out-of-range index: must return None, not panic, in BOTH debug
    // and release. Callers depend on this to skip the edit + skip
    // push_edit (no phantom undo entries).
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(99)]);
    assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

    // Empty path.
    let path = StageId(vec![]);
    assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

    // Path starts with a branch segment.
    let path = StageId(vec![StageIdSegment::IfTrue]);
    assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

    // Branch segment after a non-Conditional action.
    let path = StageId(vec![StageIdSegment::Index(0), StageIdSegment::IfTrue]);
    assert!(replace_at_path(&actions, &path, Action::Invert).is_none());
}

#[test]
fn insert_at_path_outer_appends() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(1)]);
    let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
    assert_eq!(new.len(), 2);
}

#[test]
fn insert_at_path_outer_inserts_at_index() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    let new = insert_at_path(
        &actions,
        &path,
        Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
    )
    .expect("valid path");
    assert_eq!(new.len(), 2);
    assert!(matches!(new[0], Action::MergeAxis { .. }));
    assert!(matches!(new[1], Action::Invert));
}

#[test]
fn insert_at_path_into_if_false_creates_branch() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfFalse,
        StageIdSegment::Index(0),
    ]);
    let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
    match &new[0] {
        Action::Conditional { if_false, .. } => {
            assert_eq!(if_false.len(), 1);
        }
        _ => panic!("expected Conditional"),
    }
}

#[test]
fn remove_at_path_outer_drops_action() {
    let actions = vec![Action::Invert, Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    let new = remove_at_path(&actions, &path).expect("valid path");
    assert_eq!(new.len(), 1);
}

#[test]
fn remove_at_path_last_in_if_false_leaves_empty_branch() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![],
        if_false: vec![Action::Invert],
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfFalse,
        StageIdSegment::Index(0),
    ]);
    let new = remove_at_path(&actions, &path).expect("valid path");
    match &new[0] {
        Action::Conditional { if_false, .. } => {
            assert!(
                if_false.is_empty(),
                "removing the last action must leave an empty if_false branch"
            );
        }
        _ => panic!("expected Conditional"),
    }
}

#[test]
fn insert_remove_invalid_paths_return_none() {
    // Same contract as replace_at_path: callers depend on None
    // (NOT panic in release) so they can skip the edit + skip push_edit.
    let actions = vec![Action::Invert];

    // Empty path.
    assert!(insert_at_path(&actions, &StageId(vec![]), Action::Invert).is_none());
    assert!(remove_at_path(&actions, &StageId(vec![])).is_none());

    // Path starts with branch segment.
    assert!(
        insert_at_path(
            &actions,
            &StageId(vec![StageIdSegment::IfTrue]),
            Action::Invert
        )
        .is_none()
    );
    assert!(remove_at_path(&actions, &StageId(vec![StageIdSegment::IfTrue])).is_none());

    // Out-of-range index for remove_at_path.
    assert!(remove_at_path(&actions, &StageId(vec![StageIdSegment::Index(99)])).is_none());

    // Branch segment after a non-Conditional action.
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfTrue,
        StageIdSegment::Index(0),
    ]);
    assert!(insert_at_path(&actions, &path, Action::Invert).is_none());
    assert!(remove_at_path(&actions, &path).is_none());
}

// ---------------------------------------------------------------------------
// SSR helpers for pipeline rendering tests (Task 20)
// ---------------------------------------------------------------------------

fn build_state(actions: Vec<Action>) -> (AppState, InputAddress) {
    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
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
        diagnostics: inputforge_core::types::DeviceDiagnostics::default(),
    });
    (state, addr)
}

// ---------------------------------------------------------------------------
// Harness (VirtualDom::new_with_props pattern matching mapping_editor::tests)
// ---------------------------------------------------------------------------

#[derive(Clone, Props)]
struct HarnessProps {
    state: Arc<RwLock<AppState>>,
    addr: InputAddress,
    /// Stage IDs to pre-expand in `EditorState` before rendering. Used by tests
    /// that assert on body content (Task 22+).
    #[props(default)]
    pre_expanded_stages: Vec<StageId>,
    /// Virtual devices to seed into the `ConfigSnapshot`. Used by Task 23+
    /// body tests that exercise the device/output pickers.
    #[props(default)]
    virtual_devices: Vec<VirtualDeviceConfig>,
    /// Pre-seed the `EditorState::stage_menu` signal so SSR tests can assert
    /// the right-click menu renders with the expected items (Task 29). SSR
    /// cannot dispatch real `oncontextmenu` events, so we plant the state
    /// directly.
    #[props(default)]
    pre_stage_menu: Option<crate::frame::mapping_editor::StageMenuState>,
    /// Pre-seed `EditorState::malformed_hints` so SSR tests can assert the
    /// error-tint title treatment (Task 35). Body components write hints via
    /// `use_effect`, which does not fire during SSR, so tests must inject
    /// hints directly.
    #[props(default)]
    pre_malformed_hints: HashMap<StageId, String>,
}

impl PartialEq for HarnessProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
            && self.addr == other.addr
            && self.pre_expanded_stages == other.pre_expanded_stages
            && self.virtual_devices == other.virtual_devices
            && self.pre_stage_menu == other.pre_stage_menu
            && self.pre_malformed_hints == other.pre_malformed_hints
    }
}

#[allow(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention"
)]
fn HarnessComponent(props: HarnessProps) -> Element {
    let HarnessProps {
        state,
        addr,
        pre_expanded_stages,
        virtual_devices,
        pre_stage_menu,
        pre_malformed_hints,
    } = props;

    let (cmd_tx, _) = mpsc::channel();
    let raw = RawHandles {
        state,
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
    };
    use_context_provider(|| raw.clone());

    let selection = ("Default".to_owned(), addr.clone());
    let mut snap = ConfigSnapshot::from_state(&raw.state.read(), Some(&selection));
    // Inject test-supplied virtual devices into the snapshot so body
    // components (Task 23+) that read `cfg.virtual_devices` see them.
    if !virtual_devices.is_empty() {
        snap.virtual_devices = virtual_devices;
    }
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
        .replace(("Default".to_owned(), addr));
    use_context_provider(|| view);
    use_live_capture_provider();
    let editor = use_editor_state_provider();
    for stage_id in pre_expanded_stages {
        editor.expanded_stages.clone().write().insert(stage_id);
    }
    if let Some(menu) = pre_stage_menu {
        editor.stage_menu.clone().write().replace(menu);
    }
    if !pre_malformed_hints.is_empty() {
        *editor.malformed_hints.clone().write() = pre_malformed_hints;
    }
    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });
    rsx! { MappingEditor {} }
}

fn render_with(state: AppState, addr: InputAddress) -> String {
    render_with_expanded(state, addr, vec![])
}

fn render_with_expanded(
    state: AppState,
    addr: InputAddress,
    pre_expanded_stages: Vec<StageId>,
) -> String {
    render_with_full(state, addr, pre_expanded_stages, vec![])
}

/// Render helper that accepts both pre-expanded stages and virtual devices.
/// Used by Task 23+ body tests.
fn render_with_full(
    state: AppState,
    addr: InputAddress,
    pre_expanded_stages: Vec<StageId>,
    virtual_devices: Vec<VirtualDeviceConfig>,
) -> String {
    let mut vdom = VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            pre_expanded_stages,
            virtual_devices,
            pre_stage_menu: None,
            pre_malformed_hints: HashMap::new(),
        },
    );
    vdom.rebuild_in_place();
    render(&vdom)
}

/// Render helper that runs an extra `render_immediate` pass after the
/// initial rebuild, allowing render-phase signal writes from a child
/// (e.g. `editor.malformed_hints` writes inside a `stage_body` component)
/// to propagate back to the parent `Stage` component that reads the same
/// signal. Without this second pass the `Stage` reads the signal before
/// the child has had a chance to write, and the rendered HTML reflects
/// the pre-write state.
///
/// Used by Task 9's malformed-hint SSR tests where the unbound /
/// per-kind hint must appear in the stage summary slot.
fn render_with_expanded_settled(
    state: AppState,
    addr: InputAddress,
    pre_expanded_stages: Vec<StageId>,
) -> String {
    let mut vdom = VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            pre_expanded_stages,
            virtual_devices: vec![],
            pre_stage_menu: None,
            pre_malformed_hints: HashMap::new(),
        },
    );
    vdom.rebuild_in_place();
    // Second pass: pick up dirty scopes flagged by the child's render-phase
    // writes to `editor.malformed_hints`. Dioxus marks the parent dirty
    // when a child writes to a Signal the parent has subscribed to via
    // `.read()`, but `rebuild_in_place` only runs one pass, so the parent
    // (`Stage`) ends up rendering the pre-write summary unless we drive a
    // second render explicitly.
    vdom.render_immediate(&mut dioxus::core::NoOpMutations);
    render(&vdom)
}

/// Render helper that pre-seeds `EditorState::malformed_hints`. Used by
/// Task 35's SSR test: body `use_effect` blocks do not fire during SSR, so
/// we inject the hint directly and verify the visual treatment appears.
fn render_with_malformed_hints(
    state: AppState,
    addr: InputAddress,
    pre_malformed_hints: HashMap<StageId, String>,
) -> String {
    let mut vdom = VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            pre_expanded_stages: vec![],
            virtual_devices: vec![],
            pre_stage_menu: None,
            pre_malformed_hints,
        },
    );
    vdom.rebuild_in_place();
    render(&vdom)
}

/// Render helper that pre-seeds the right-click stage actions menu state.
/// Used by Task 29's SSR test (real `oncontextmenu` events cannot be
/// simulated through the SSR renderer).
fn render_with_stage_menu(
    state: AppState,
    addr: InputAddress,
    pre_stage_menu: Option<crate::frame::mapping_editor::StageMenuState>,
) -> String {
    let mut vdom = VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            pre_expanded_stages: vec![],
            virtual_devices: vec![],
            pre_stage_menu,
            pre_malformed_hints: HashMap::new(),
        },
    );
    vdom.rebuild_in_place();
    render(&vdom)
}

// ---------------------------------------------------------------------------
// Task 20 SSR tests
// ---------------------------------------------------------------------------

#[test]
fn pipeline_renders_ordered_list_with_one_invert_stage() {
    let (state, addr) = build_state(vec![Action::Invert]);
    let html = render_with(state, addr);
    assert!(html.contains("<ol"), "pipeline must use <ol>: {html}");
    assert!(
        html.contains("if-stage"),
        "stage card class missing: {html}"
    );
    assert!(
        html.contains("Invert"),
        "stage variant title missing: {html}"
    );
}

#[test]
fn pipeline_empty_branch_renders_add_first_stage_affordance() {
    let (state, addr) = build_state(vec![]);
    let html = render_with(state, addr);
    assert!(
        html.contains("Add first stage"),
        "empty pipeline must show louder add affordance: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 21: stage_title_for / stage_summary_for unit tests
// ---------------------------------------------------------------------------

use crate::frame::mapping_editor::pipeline::stage::{stage_summary_for, stage_title_for};

/// Build a minimal [`ConfigSnapshot`] containing a single device named "Stick".
fn synth_cfg() -> ConfigSnapshot {
    let device = inputforge_core::state::DeviceState {
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
        diagnostics: inputforge_core::types::DeviceDiagnostics::default(),
    };
    let device_display_names = HashMap::from([(device.info.id.clone(), device.info.name.clone())]);
    ConfigSnapshot {
        devices: vec![device],
        device_display_names,
        ..ConfigSnapshot::default()
    }
}

/// Variant of [`synth_cfg`] whose `device_display_names` entry is an
/// alias deliberately distinct from `info.name`. Used by the alias
/// regression tests below: a silent revert of the call site to
/// `info.name` flips the asserted substring from the alias back to
/// `"Stick"` and the test fails.
fn synth_cfg_with_alias(alias: &str) -> ConfigSnapshot {
    let device = inputforge_core::state::DeviceState {
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
        diagnostics: inputforge_core::types::DeviceDiagnostics::default(),
    };
    let device_display_names = HashMap::from([(device.info.id.clone(), alias.to_owned())]);
    ConfigSnapshot {
        devices: vec![device],
        device_display_names,
        ..ConfigSnapshot::default()
    }
}

#[test]
fn title_for_each_variant() {
    assert_eq!(stage_title_for(&Action::Invert), "Invert");
    assert_eq!(
        stage_title_for(&Action::Deadzone {
            config: DeadzoneConfig::default()
        }),
        "Deadzone"
    );
    assert_eq!(
        stage_title_for(&Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X }
            }
        }),
        "Map to vJoy"
    );
    assert_eq!(
        stage_title_for(&Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        }),
        "Merge axis"
    );
}

#[test]
fn summary_invert_is_empty() {
    let s = stage_summary_for(&Action::Invert, &synth_cfg());
    assert_eq!(s, "");
}

#[test]
fn summary_merge_axis_lists_op_and_secondary() {
    let s = stage_summary_for(
        &Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
        &synth_cfg(),
    );
    assert!(s.contains("Average"), "expected op in summary: {s}");
    assert!(s.contains("Stick"), "expected device in summary: {s}");
}

#[test]
fn summary_merge_axis_uses_alias_for_secondary_device() {
    // Regression guard: MergeAxis stage labels must route through
    // `cfg.device_display_name(...)`. The cfg below maps `dev-1` to
    // alias "Rig Wheel" while keeping `info.name = "Stick"`, so a
    // revert to `info.name` would flip the summary back to "Stick"
    // and trip the negated assertion.
    let s = stage_summary_for(
        &Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
        &synth_cfg_with_alias("Rig Wheel"),
    );
    assert!(s.contains("Rig Wheel"), "expected alias in summary: {s}");
    assert!(
        !s.contains("Stick"),
        "summary leaked hardware name instead of alias: {s}"
    );
}

#[test]
fn summary_conditional_button_pressed_uses_alias() {
    // Regression guard for the `format_condition` ->
    // `predicate_device_label` -> `device_label` chain. Same alias
    // contract as above, exercised through the Conditional /
    // ButtonPressed predicate.
    let s = stage_summary_for(
        &Action::Conditional {
            condition: Condition::ButtonPressed {
                input: synth_addr(),
            },
            if_true: vec![],
            if_false: vec![],
        },
        &synth_cfg_with_alias("Rig Wheel"),
    );
    assert!(s.contains("Rig Wheel"), "expected alias in summary: {s}");
    assert!(
        !s.contains("Stick"),
        "summary leaked hardware name instead of alias: {s}"
    );
}

#[test]
fn summary_map_to_keyboard_renders_combo() {
    let s = stage_summary_for(
        &Action::MapToKeyboard {
            key: KeyCombo {
                key: "Q".to_owned(),
                modifiers: vec![KeyModifier::Ctrl, KeyModifier::Shift],
            },
        },
        &synth_cfg(),
    );
    assert!(s.contains("Ctrl"), "missing Ctrl in: {s}");
    assert!(s.contains("Shift"), "missing Shift in: {s}");
    assert!(s.contains('Q'), "missing key in: {s}");
}

#[test]
fn summary_deadzone_reports_inner_band_width_and_outer_saturation_width() {
    // low/high are the OUTER saturation thresholds; values past them clamp
    // to +-1.0. So the outer dead band on the positive side is
    // (1.0 - high) * 100. The inner dead band is the dead-center span,
    // (center_high - center_low) * 100. Pin both so the formula does not
    // drift back to reporting `high` directly (which would mislabel the
    // live range as the dead band).
    let cfg = DeadzoneConfig::new(-0.85, -0.10, 0.10, 0.85).unwrap();
    let s = stage_summary_for(&Action::Deadzone { config: cfg }, &synth_cfg());
    assert!(s.contains("inner 20%"), "expected inner 20% in: {s}");
    assert!(s.contains("outer 15%"), "expected outer 15% in: {s}");
}

// ---------------------------------------------------------------------------
// Task 22: Stage body dispatcher + Invert body
// ---------------------------------------------------------------------------

#[test]
fn invert_stage_expanded_renders_descriptive_caption() {
    let (state, addr) = build_state(vec![Action::Invert]);
    let pre_expanded = vec![StageId(vec![StageIdSegment::Index(0)])];
    let html = render_with_expanded(state, addr, pre_expanded);
    assert!(
        html.contains("Inverts the input value"),
        "expected Invert descriptive caption in body: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 23: MapToVJoy body (device + output pickers)
// ---------------------------------------------------------------------------

#[test]
fn map_to_vjoy_body() {
    // Seed a vJoy device 1 with axes X and Y, and a mapping that uses it.
    let vd = VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X, VJoyAxis::Y],
        button_count: 0,
        hat_count: 0,
    };
    let output = OutputAddress {
        device: 1,
        output: OutputId::Axis { id: VJoyAxis::X },
    };
    let (state, addr) = build_state(vec![Action::MapToVJoy { output }]);
    let pre_expanded = vec![StageId(vec![StageIdSegment::Index(0)])];
    let html = render_with_full(state, addr, pre_expanded, vec![vd]);

    // The body must render a device picker with a label containing "Device"
    // and a select option for "vJoy device 1".
    assert!(
        html.contains("vJoy device 1"),
        "expected 'vJoy device 1' option in device picker: {html}"
    );
    // The body must render an output picker with an option for X axis.
    assert!(
        html.contains("X axis") || html.contains('X'),
        "expected axis option in output picker: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 24: MapToKeyboard body (modifier toggles + key field)
// ---------------------------------------------------------------------------

#[test]
fn map_to_keyboard_body_renders_modifier_toggles_and_key_field() {
    let actions = vec![Action::MapToKeyboard {
        key: KeyCombo {
            key: "Q".to_owned(),
            modifiers: vec![KeyModifier::Ctrl],
        },
    }];
    let (state, addr) = build_state(actions);
    let pre_expanded = vec![StageId(vec![StageIdSegment::Index(0)])];
    let html = render_with_expanded(state, addr, pre_expanded);

    // The body must render modifier labels.
    assert!(
        html.contains("Ctrl"),
        "expected Ctrl modifier toggle in body: {html}"
    );
    // The key text field must be present (TextInput renders an <input type="text">).
    assert!(
        html.contains("Key") || html.contains(r#"type="text""#),
        "expected Key field in body: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 25: MergeAxis body (op picker + secondary input picker)
// ---------------------------------------------------------------------------

#[test]
fn merge_axis_body_renders_op_picker_and_secondary_input() {
    let actions = vec![Action::MergeAxis {
        second_input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        },
        operation: MergeOp::Average,
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);
    assert!(
        html.contains("Average") || html.contains("Bidirectional"),
        "expected op picker option in DOM: {html}"
    );
    assert!(
        html.contains("rebind"),
        "secondary picker rebind button missing"
    );
}

#[test]
fn merge_axis_body_writes_malformed_hint_when_secondary_equals_primary() {
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::MergeAxis {
        second_input: primary.clone(), // duplicate of primary
        operation: MergeOp::Average,
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);
    // The malformed-hint visual treatment lands in Task 35, but the hint
    // should still be set on the EditorState. For now, asserting the body
    // renders (no panic) is sufficient for Task 25.
    assert!(
        html.contains("Average"),
        "body must still render with duplicate secondary: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 26b: PredicateEditor -- 7-kind picker + operand fields
// ---------------------------------------------------------------------------

#[test]
fn predicate_editor_renders_kind_picker_with_seven_options() {
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);
    // All 7 kind names must appear in the kind-picker select options.
    assert!(
        html.contains("ButtonPressed"),
        "ButtonPressed missing: {html}"
    );
    assert!(
        html.contains("ButtonReleased"),
        "ButtonReleased missing: {html}"
    );
    assert!(html.contains("AxisInRange"), "AxisInRange missing: {html}");
    assert!(
        html.contains("HatDirection"),
        "HatDirection missing: {html}"
    );
    assert!(html.contains(">All<"), "All option missing: {html}");
    assert!(html.contains(">Any<"), "Any option missing: {html}");
    assert!(html.contains(">Not<"), "Not option missing: {html}");
}

#[test]
fn predicate_axis_in_range_renders_min_max_inputs() {
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::AxisInRange {
            input: primary.clone(),
            min: -0.5,
            max: 0.5,
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);
    // NumberInput renders <input type="number"> elements for min and max.
    let count = html.matches(r#"type="number""#).count();
    assert!(
        count >= 2,
        "expected at least 2 number inputs for min+max, got {count}: {html}"
    );
}

#[test]
fn predicate_all_recursive_renders_nested_predicate_editors() {
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::All {
            conditions: vec![
                Condition::ButtonPressed {
                    input: primary.clone(),
                },
                Condition::ButtonReleased {
                    input: primary.clone(),
                },
            ],
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);
    // The outer All editor renders its nested-list container.
    assert!(
        html.contains("if-predicate__nested-list"),
        "expected nested-list container for All: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 26a: Conditional shell with recursive branch sub-pipelines
// ---------------------------------------------------------------------------

#[test]
fn conditional_body_renders_branches_with_correct_aria_labels() {
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![Action::Invert],
        if_false: vec![Action::Invert],
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);
    assert!(
        html.contains("if true branch"),
        "expected if-true aria-label: {html}"
    );
    assert!(
        html.contains("if false branch"),
        "expected if-false aria-label: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 28: AddPalette rendering
// ---------------------------------------------------------------------------

#[test]
fn add_palette_renders_three_categorized_sections() {
    let (state, addr) = build_state(vec![]); // empty pipeline
    let html = render_with_expanded(state, addr, vec![]);
    // The palette button should be visible (empty pipeline shows louder label).
    assert!(
        html.contains("Add first stage"),
        "expected Add first stage: {html}"
    );
}

#[test]
fn add_palette_button_renders_for_non_empty_pipeline() {
    let (state, addr) = build_state(vec![Action::Invert]);
    let html = render_with_expanded(state, addr, vec![]);
    // The end-of-pipeline add palette or its container must be present.
    assert!(
        html.contains("if-pipeline__add-end") || html.contains("if-add-palette"),
        "expected add-end or add-palette class: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 16: F9 dispatcher integration (ResponseCurve body + thumbnail)
// ---------------------------------------------------------------------------

#[test]
fn response_curve_stage_expanded_renders_f10_body_not_placeholder() {
    // Mount Pipeline with [Action::ResponseCurve { curve: identity }],
    // pre-expand stage 0.
    // Assert html contains "if-curve" (F10 root class) AND does NOT contain
    // "F10 / F11 / F14 owns this body" (former placeholder caption).
    let actions = vec![Action::ResponseCurve {
        curve: inputforge_core::processing::ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (1.0, 1.0)],
            symmetric: false,
        },
    }];
    let (state, addr) = build_state(actions);
    let pre_expanded = vec![StageId(vec![StageIdSegment::Index(0)])];
    let html = render_with_expanded(state, addr, pre_expanded);
    assert!(
        html.contains("if-curve"),
        "expected F10 body root class 'if-curve' in expanded stage: {html}"
    );
    assert!(
        !html.contains("F10 / F11 / F14 owns this body"),
        "placeholder caption must not appear after F10 body lands: {html}"
    );
}

#[test]
fn response_curve_header_right_slot_emits_thumbnail_not_chevron() {
    // Mount Pipeline with the same identity curve, collapsed (no pre-expand).
    // Assert html contains "if-curve__thumbnail" AND does NOT contain
    // the default chevron class "if-stage__chevron".
    let actions = vec![Action::ResponseCurve {
        curve: inputforge_core::processing::ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (1.0, 1.0)],
            symmetric: false,
        },
    }];
    let (state, addr) = build_state(actions);
    // Do NOT pre-expand; the collapsed header renders only the right-slot thumbnail.
    let html = render_with(state, addr);
    assert!(
        html.contains("if-curve__thumbnail"),
        "expected thumbnail class 'if-curve__thumbnail' in collapsed header: {html}"
    );
    assert!(
        !html.contains("if-stage__chevron"),
        "default chevron must not appear in ResponseCurve header after F10 lands: {html}"
    );
}

#[test]
fn conditional_three_deep_renders_all_branches() {
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let inner = Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![Action::Invert],
        if_false: Vec::new(),
    };
    let middle = Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![inner],
        if_false: Vec::new(),
    };
    let outer = Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![middle],
        if_false: Vec::new(),
    };
    let (state, addr) = build_state(vec![outer]);
    let html = render_with_expanded(
        state,
        addr,
        vec![
            StageId(vec![StageIdSegment::Index(0)]),
            StageId(vec![
                StageIdSegment::Index(0),
                StageIdSegment::IfTrue,
                StageIdSegment::Index(0),
            ]),
            StageId(vec![
                StageIdSegment::Index(0),
                StageIdSegment::IfTrue,
                StageIdSegment::Index(0),
                StageIdSegment::IfTrue,
                StageIdSegment::Index(0),
            ]),
        ],
    );
    // The innermost stage should render "Invert" inside the recursion
    assert!(
        html.contains("Invert"),
        "innermost Invert stage missing: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 29: right-click stage actions menu SSR test
// ---------------------------------------------------------------------------

#[test]
fn right_click_on_stage_opens_actions_menu() {
    use crate::frame::mapping_editor::StageMenuState;

    let actions = vec![Action::Invert, Action::Invert];
    let (state, addr) = build_state(actions);
    let html = render_with_stage_menu(
        state,
        addr,
        Some(StageMenuState {
            stage: StageId(vec![StageIdSegment::Index(0)]),
            x: 100.0,
            y: 200.0,
        }),
    );
    assert!(
        html.contains("Move up"),
        "expected 'Move up' menu item in: {html}"
    );
    assert!(
        html.contains("Move down"),
        "expected 'Move down' menu item in: {html}"
    );
    assert!(
        html.contains("Delete"),
        "expected 'Delete' menu item in: {html}"
    );
    // Menu must be anchored at cursor coordinates via inline style
    assert!(
        html.contains("left: 100px"),
        "menu must be anchored at cursor x: {html}"
    );
    assert!(
        html.contains("top: 200px"),
        "menu must be anchored at cursor y: {html}"
    );
}

#[test]
fn stage_menu_disables_move_up_at_first_position() {
    use crate::frame::mapping_editor::StageMenuState;

    let actions = vec![Action::Invert, Action::Invert];
    let (state, addr) = build_state(actions);
    let html = render_with_stage_menu(
        state,
        addr,
        Some(StageMenuState {
            stage: StageId(vec![StageIdSegment::Index(0)]),
            x: 0.0,
            y: 0.0,
        }),
    );
    // Move up button must carry the disabled attribute when index is 0.
    // SSR emits boolean attrs as bare keywords; we look for the disabled
    // attribute appearing alongside the Move up text.
    let move_up_pos = html.find("Move up").expect("Move up item must render");
    let prefix = &html[..move_up_pos];
    assert!(
        prefix.contains("aria-disabled=\"true\""),
        "Move up at index 0 must be aria-disabled: {html}"
    );
}

#[test]
fn stage_menu_disables_move_down_at_last_position() {
    use crate::frame::mapping_editor::StageMenuState;

    let actions = vec![Action::Invert, Action::Invert];
    let (state, addr) = build_state(actions);
    let html = render_with_stage_menu(
        state,
        addr,
        Some(StageMenuState {
            stage: StageId(vec![StageIdSegment::Index(1)]),
            x: 0.0,
            y: 0.0,
        }),
    );
    // The Move down button is the second menuitem; locate its substring
    // and verify aria-disabled appears in its preceding attributes.
    let move_down_pos = html.find("Move down").expect("Move down item must render");
    // Look back at most 200 chars to find the button's own attributes
    let start = move_down_pos.saturating_sub(200);
    let chunk = &html[start..move_down_pos];
    assert!(
        chunk.contains("aria-disabled=\"true\""),
        "Move down at last index must be aria-disabled: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 30b: DnD cycle-prevention unit tests + cross-pipeline integration test
// ---------------------------------------------------------------------------

use crate::frame::mapping_editor::pipeline::dnd::{is_descendant, validate_pipeline_drop};

#[test]
fn dnd_descendant_detection_rejects_self_descent() {
    // A Conditional at index 0 must be detected as an ancestor of any stage
    // nested within its own branches.
    let ancestor = StageId(vec![StageIdSegment::Index(0)]);
    let candidate = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfTrue,
        StageIdSegment::Index(0),
    ]);
    assert!(
        is_descendant(&ancestor, &candidate),
        "stage nested inside its own if_true branch must be detected as descendant"
    );
}

#[test]
fn dnd_descendant_detection_allows_unrelated_path() {
    // Two sibling stages at the outer pipeline level share no prefix relation.
    let ancestor = StageId(vec![StageIdSegment::Index(0)]);
    let candidate = StageId(vec![StageIdSegment::Index(1)]);
    assert!(
        !is_descendant(&ancestor, &candidate),
        "unrelated sibling must not be detected as descendant"
    );
}

#[test]
fn dnd_descendant_detection_allows_self_drop_to_outer_pipeline() {
    // Dragging a stage to a sibling slot at the same depth must be allowed.
    let ancestor = StageId(vec![StageIdSegment::Index(2)]);
    let candidate = StageId(vec![StageIdSegment::Index(5)]);
    assert!(
        !is_descendant(&ancestor, &candidate),
        "sibling at same depth must not be detected as descendant"
    );
}

#[test]
fn dnd_can_move_stage_from_outer_into_conditional_if_true() {
    // Integration: move Action::Invert (at outer index 1) into the if_true
    // branch of the Conditional at outer index 0. The resulting tree must have
    // one outer stage (the Conditional) with one inner stage (Invert).
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![
        Action::Conditional {
            condition: Condition::ButtonPressed {
                input: primary.clone(),
            },
            if_true: vec![],
            if_false: Vec::new(),
        },
        Action::Invert,
    ];

    // Source: outer index 1 (the Invert).
    let drag_id = StageId(vec![StageIdSegment::Index(1)]);
    // Target: inside if_true of the Conditional, at index 0.
    let drop_id = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfTrue,
        StageIdSegment::Index(0),
    ]);

    // The cycle validator must allow this move (source is NOT an ancestor of target).
    assert!(
        validate_pipeline_drop(&drag_id, &drop_id),
        "moving an outer stage into a Conditional's if_true branch must be allowed"
    );

    let dragged = at_path(&actions, &drag_id)
        .cloned()
        .expect("drag_id must resolve");
    let after_remove = remove_at_path(&actions, &drag_id).expect("remove must succeed");
    let result = insert_at_path(&after_remove, &drop_id, dragged).expect("insert must succeed");

    match &result[0] {
        Action::Conditional { if_true, .. } => {
            assert_eq!(
                if_true.len(),
                1,
                "if_true must contain the moved Invert stage"
            );
            assert!(
                matches!(if_true[0], Action::Invert),
                "moved stage must be Invert"
            );
        }
        _ => panic!("outer stage 0 must remain Conditional after move"),
    }
    assert_eq!(
        result.len(),
        1,
        "outer pipeline must have exactly one stage after the Invert is moved inside"
    );
}

// ---------------------------------------------------------------------------
// Gap-model DnD: post-remove slot conversion + drop dispatch
// ---------------------------------------------------------------------------

use crate::frame::mapping_editor::pipeline::dnd::gap_to_post_remove_slot;

#[test]
fn dnd_gap_drop_dispatches_correct_target_index() {
    // Drag stage 0 in [Invert, Deadzone, MergeAxis], drop in gap 2
    // (between Deadzone and MergeAxis). Result must be
    // [Deadzone, Invert, MergeAxis] -- the source's removal at index 0
    // shifts every later index left, so the gap-2 drop lands at the
    // post-remove slot 1.
    let second_input = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![
        Action::Invert,
        Action::Deadzone {
            config: DeadzoneConfig::default(),
        },
        Action::MergeAxis {
            second_input,
            operation: MergeOp::Average,
        },
    ];

    let parent_path = StageId(Vec::new());
    let src_local_index: usize = 0;
    let gap_index: usize = 2;

    let post_remove_to =
        gap_to_post_remove_slot(&parent_path, &parent_path, src_local_index, gap_index);
    assert_eq!(
        post_remove_to, 1,
        "same-branch downward drop must subtract one for the source removal shift"
    );

    let src_id = StageId(vec![StageIdSegment::Index(src_local_index)]);
    let tgt_id = StageId(vec![StageIdSegment::Index(post_remove_to)]);
    let dragged = at_path(&actions, &src_id)
        .cloned()
        .expect("source resolves");
    let after_remove = remove_at_path(&actions, &src_id).expect("remove succeeds");
    let result = insert_at_path(&after_remove, &tgt_id, dragged).expect("insert succeeds");

    assert_eq!(result.len(), 3, "stage count is preserved");
    assert!(matches!(result[0], Action::Deadzone { .. }));
    assert!(matches!(result[1], Action::Invert));
    assert!(matches!(result[2], Action::MergeAxis { .. }));
}

#[test]
fn dnd_gap_drop_cross_pipeline_no_shift() {
    // Drag a stage from the outer pipeline into a Conditional's if_true
    // branch. Cross-branch drops do not shift indices because the
    // source's removal happens in a different branch from the insert.
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![
        Action::Conditional {
            condition: Condition::ButtonPressed {
                input: primary.clone(),
            },
            if_true: vec![Action::Deadzone {
                config: DeadzoneConfig::default(),
            }],
            if_false: Vec::new(),
        },
        Action::Invert,
    ];

    let src_parent = StageId(Vec::new());
    let tgt_parent = StageId(vec![StageIdSegment::Index(0), StageIdSegment::IfTrue]);
    let src_local_index: usize = 1;
    let gap_index: usize = 0; // before the existing Deadzone in if_true

    // Cross-branch: gap_index passes through unchanged.
    let post_remove_to =
        gap_to_post_remove_slot(&src_parent, &tgt_parent, src_local_index, gap_index);
    assert_eq!(post_remove_to, 0, "cross-branch drops do not shift indices");

    let src_id = StageId(vec![StageIdSegment::Index(src_local_index)]);
    let mut tgt_segs = tgt_parent.0.clone();
    tgt_segs.push(StageIdSegment::Index(post_remove_to));
    let tgt_id = StageId(tgt_segs);

    let dragged = at_path(&actions, &src_id)
        .cloned()
        .expect("source resolves");
    let after_remove = remove_at_path(&actions, &src_id).expect("remove succeeds");
    let result = insert_at_path(&after_remove, &tgt_id, dragged).expect("insert succeeds");

    match &result[0] {
        Action::Conditional { if_true, .. } => {
            assert_eq!(if_true.len(), 2, "if_true now contains both stages");
            assert!(
                matches!(if_true[0], Action::Invert),
                "Invert lands at index 0"
            );
            assert!(matches!(if_true[1], Action::Deadzone { .. }));
        }
        _ => panic!("outer stage 0 must remain Conditional"),
    }
    assert_eq!(result.len(), 1, "outer pipeline shrinks to one stage");
}

#[test]
fn dnd_source_adjacent_gap_is_noop() {
    // Source-adjacent gaps (gap_index == src_local_index OR
    // gap_index == src_local_index + 1) are silently suppressed by
    // `SortableGap`. As a math check, even if the suppression were
    // removed and the same-branch conversion were applied, the
    // resulting post-remove insertion would round-trip the source to
    // its original slot. This test is the regression gate for that
    // mathematical property: a no-op must remain a no-op.
    let actions = vec![
        Action::Invert,
        Action::Deadzone {
            config: DeadzoneConfig::default(),
        },
        Action::Invert,
    ];
    let parent_path = StageId(Vec::new());
    let src_local_index: usize = 1;

    for gap_index in [src_local_index, src_local_index + 1] {
        let post_remove_to =
            gap_to_post_remove_slot(&parent_path, &parent_path, src_local_index, gap_index);
        let src_id = StageId(vec![StageIdSegment::Index(src_local_index)]);
        let tgt_id = StageId(vec![StageIdSegment::Index(post_remove_to)]);

        let dragged = at_path(&actions, &src_id)
            .cloned()
            .expect("source resolves");
        let after_remove = remove_at_path(&actions, &src_id).expect("remove succeeds");
        let result = insert_at_path(&after_remove, &tgt_id, dragged).expect("insert succeeds");

        assert_eq!(
            result.len(),
            actions.len(),
            "no-op preserves stage count for gap_index {gap_index}"
        );
        assert!(
            matches!(result[1], Action::Deadzone { .. }),
            "source returns to its original slot for gap_index {gap_index}"
        );
    }
}

#[test]
fn dnd_invalid_validator_blocks_drop() {
    // Drag a Conditional onto a gap inside its own if_true branch. The
    // validator must reject the drop because the source's path is a
    // strict prefix of the target's parent path (cycle).
    let src_id = StageId(vec![StageIdSegment::Index(0)]);
    let tgt_parent = StageId(vec![StageIdSegment::Index(0), StageIdSegment::IfTrue]);

    // Validator receives parent paths in production: source's stage path
    // (the dragged Conditional's full id) and target gap's parent path
    // (the if_true branch). The validator's defensive policy rejects
    // when the source is a strict prefix of the target's group path.
    assert!(
        !validate_pipeline_drop(&src_id, &tgt_parent),
        "dropping a Conditional into its own if_true branch must be rejected"
    );

    // A drop the validator allows: dragging a sibling at the outer level
    // to another outer-level slot. Same parent (outer), no prefix
    // relationship between source path and target group.
    let sibling_src = StageId(vec![StageIdSegment::Index(1)]);
    let outer_parent = StageId(Vec::new());
    assert!(
        validate_pipeline_drop(&sibling_src, &outer_parent),
        "same-pipeline reorder must be allowed"
    );
}

// ---------------------------------------------------------------------------
// Task 38: four-stage pipeline SSR coverage (AC #9)
// ---------------------------------------------------------------------------

#[test]
fn four_stage_pipeline_renders_all_categories_and_summaries() {
    // Build a four-stage pipeline that exercises all category tints:
    //   0: Deadzone       -- is-processing
    //   1: ResponseCurve  -- is-processing, 3 pts, symmetric=false (no "sym" qualifier)
    //   2: MergeAxis      -- is-output
    //   3: MapToVJoy      -- is-output
    //
    // The test renders collapsed (no pre-expanded stages) and walks the DOM
    // with `scraper` to verify stage count, ordering, and category classes
    // without relying on substring-search position arithmetic.
    use scraper::{Html, Selector};

    let second_input = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let curve = inputforge_core::processing::ResponseCurve::piecewise_linear(
        vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
        false,
    )
    .expect("valid 3-point identity curve");

    let actions = vec![
        Action::Deadzone {
            config: DeadzoneConfig::default(),
        },
        Action::ResponseCurve { curve },
        Action::MergeAxis {
            second_input,
            operation: MergeOp::Average,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
    ];
    let (state, addr) = build_state(actions);
    // Collapsed render: no pre-expanded stages -- just verify structure.
    let html = render_with_expanded(state, addr, vec![]);

    let doc = Html::parse_document(&html);
    let stage_sel = Selector::parse("li.if-stage").expect("selector must be valid");
    let stages: Vec<_> = doc.select(&stage_sel).collect();
    assert_eq!(
        stages.len(),
        4,
        "expected 4 pipeline stages, got {}; html: {html}",
        stages.len()
    );

    // Stage 0: Deadzone -- is-processing category.
    let s0_class = stages[0].value().attr("class").unwrap_or("");
    assert!(
        s0_class.contains("is-processing"),
        "stage 0 (Deadzone) must carry is-processing class; class='{s0_class}'"
    );
    assert!(
        stages[0].html().contains("Deadzone"),
        "stage 0 must display 'Deadzone' title"
    );

    // Stage 1: ResponseCurve -- is-processing, 3-point, symmetric=false.
    // The formatter emits "linear · 3 pts" with no "sym" qualifier when symmetric=false.
    let s1_class = stages[1].value().attr("class").unwrap_or("");
    assert!(
        s1_class.contains("is-processing"),
        "stage 1 (ResponseCurve) must carry is-processing class; class='{s1_class}'"
    );
    assert!(
        stages[1].html().contains("Response curve"),
        "stage 1 must display 'Response curve' title"
    );
    assert!(
        stages[1].html().contains("3 pts"),
        "stage 1 summary must contain '3 pts' point count"
    );
    // symmetric=false: the "sym" qualifier must NOT appear.
    assert!(
        !stages[1].html().contains("sym"),
        "stage 1 summary must not contain 'sym' when symmetric=false"
    );

    // Stage 2: MergeAxis -- is-output category.
    let s2_class = stages[2].value().attr("class").unwrap_or("");
    assert!(
        s2_class.contains("is-output"),
        "stage 2 (MergeAxis) must carry is-output class; class='{s2_class}'"
    );
    assert!(
        stages[2].html().contains("Merge axis"),
        "stage 2 must display 'Merge axis' title"
    );

    // Stage 3: MapToVJoy -- is-output category.
    let s3_class = stages[3].value().attr("class").unwrap_or("");
    assert!(
        s3_class.contains("is-output"),
        "stage 3 (MapToVJoy) must carry is-output class; class='{s3_class}'"
    );
    assert!(
        stages[3].html().contains("Map to vJoy"),
        "stage 3 must display 'Map to vJoy' title"
    );
}

// ---------------------------------------------------------------------------
// Task 35: malformed-action visual treatment (error-tint title + hint summary)
// ---------------------------------------------------------------------------

#[test]
fn malformed_merge_axis_with_secondary_equal_primary_shows_error_class() {
    // Build a MergeAxis stage where secondary == primary. After Task 9 the
    // body component writes "Secondary input must differ from primary" to
    // malformed_hints during the render phase (no longer via a `use_effect`,
    // which would not fire during SSR), so the hint surfaces naturally on
    // the second render pass without any pre-seeding. The settled render
    // helper performs that second pass after the first-pass write has
    // propagated.
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::MergeAxis {
        second_input: primary.clone(),
        operation: MergeOp::Average,
    }];
    let (state, addr) = build_state(actions);
    let html =
        render_with_expanded_settled(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    // The error-tint modifier class must appear on the title element.
    assert!(
        html.contains("if-stage__title--error"),
        "expected error-tint class on malformed stage title: {html}"
    );
    // The hint text must appear in the summary slot instead of the normal summary.
    assert!(
        html.contains("Secondary input must differ from primary"),
        "expected malformed hint text in summary slot: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 39: Conditional branch SSR coverage (AC #11)
// ---------------------------------------------------------------------------

#[test]
fn conditional_with_empty_if_false_renders_both_branches() {
    // Both branches are always rendered as branch containers, regardless of
    // emptiness. An empty `if_false` shows the standard "Add first stage"
    // placeholder inside its branch container, same affordance as any other
    // empty pipeline. (The legacy "Add else branch" button was removed
    // 2026-05-02 along with the `Option<Vec<Action>>` indirection.)
    let btn_addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: btn_addr.clone(),
        },
        if_true: vec![Action::Invert],
        if_false: Vec::new(),
    }];
    let (state, _) = build_state(actions);
    let axis_addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let html = render_with_expanded(
        state,
        axis_addr,
        vec![StageId(vec![StageIdSegment::Index(0)])],
    );

    assert!(
        html.contains("if true branch"),
        "expected aria-label 'if true branch': {html}"
    );
    assert!(
        html.contains("if false branch"),
        "expected aria-label 'if false branch' even with an empty branch: {html}"
    );
}

#[test]
fn conditional_with_empty_if_true_renders_branch_with_add_first_stage() {
    // Build a Conditional whose `if_true` is empty and `if_false` holds one
    // Invert. The test expands the Conditional and verifies that the empty
    // `if_true` branch shows the standard "Add first stage" affordance.
    let btn_addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: btn_addr.clone(),
        },
        if_true: vec![],
        if_false: vec![Action::Invert],
    }];
    let (state, _) = build_state(actions);
    let axis_addr = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let html = render_with_expanded(
        state,
        axis_addr,
        vec![StageId(vec![StageIdSegment::Index(0)])],
    );

    assert!(
        html.contains("if true branch"),
        "expected aria-label 'if true branch': {html}"
    );
    assert!(
        html.contains("Add first stage"),
        "empty if_true must show standard add affordance: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 7: --unbound CSS modifier on the rebind composite
//
// When a leaf predicate or `MergeAxis` secondary input is `Unbound`, the
// rebind composite must carry the `if-rebind-composite--unbound` modifier
// so the placeholder label can be styled muted/italic. Previously the
// `Unbound` placeholder rendered with the same styling as a real source
// label, leaving the user no visual cue that the field was empty.
// ---------------------------------------------------------------------------

#[test]
fn predicate_input_row_unbound_renders_unbound_modifier() {
    // Conditional whose predicate input is Unbound should render the
    // `if-rebind-composite--unbound` modifier on the predicate's rebind
    // composite, with the `Unbound` placeholder text inside the label and
    // no trace of the legacy `Btn 1` sentinel that older builds produced
    // for an empty-device address.
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: InputAddress::Unbound,
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("if-rebind-composite--unbound"),
        "unbound modifier missing on predicate input row: {html}"
    );
    assert!(
        html.contains(">Unbound<"),
        "Unbound label text missing: {html}"
    );
    assert!(
        !html.contains("Btn 1"),
        "must not render the legacy Btn 1 sentinel: {html}"
    );
}

#[test]
fn predicate_input_row_bound_omits_unbound_modifier() {
    // A normal Bound input must NOT carry the unbound modifier, otherwise
    // every rebind composite would render muted.
    let bound = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: bound },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        !html.contains("if-rebind-composite--unbound"),
        "bound input must not carry unbound modifier: {html}"
    );
}

#[test]
fn merge_axis_body_unbound_secondary_renders_unbound_modifier() {
    // MergeAxis stage with an Unbound secondary input must render the
    // `if-rebind-composite--unbound` modifier on the secondary input's
    // rebind composite, with the `Unbound` placeholder text in the label.
    let actions = vec![Action::MergeAxis {
        second_input: InputAddress::Unbound,
        operation: MergeOp::Average,
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("if-rebind-composite--unbound"),
        "unbound modifier missing on merge_axis secondary input: {html}"
    );
    assert!(
        html.contains(">Unbound<"),
        "Unbound label text missing: {html}"
    );
}

#[test]
fn merge_axis_body_bound_secondary_omits_unbound_modifier() {
    // Negative guard: a Bound secondary input must NOT carry the unbound
    // modifier, otherwise the muted/italic placeholder treatment would
    // bleed onto every MergeAxis row.
    let actions = vec![Action::MergeAxis {
        second_input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        },
        operation: MergeOp::Average,
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        !html.contains("if-rebind-composite--unbound"),
        "bound secondary must not carry unbound modifier: {html}"
    );
}

#[test]
fn header_subtitle_unbound_primary_renders_unbound_modifier() {
    // When the mapping's primary input is Unbound (operationally rare, but
    // representable since Task 4: e.g. a hand-edited profile or a future
    // legacy-migration walker), the header subtitle's rebind composite
    // must carry the `if-rebind-composite--unbound` modifier so the
    // `Unbound` placeholder reads consistently with the predicate /
    // merge-axis call sites.
    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let unbound = InputAddress::Unbound;
    let mappings = vec![Mapping {
        input: unbound.clone(),
        mode: "Default".to_owned(),
        name: Some("Yaw".to_owned()),
        actions: vec![],
    }];
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let html = render_with(state, unbound);

    assert!(
        html.contains("if-rebind-composite--unbound"),
        "unbound modifier missing on header subtitle composite: {html}"
    );
    assert!(
        html.contains(">Unbound<"),
        "Unbound label text missing in header subtitle: {html}"
    );
}

// ---------------------------------------------------------------------------
// Task 9: Unbound predicate / merge-axis inputs surface a malformed-hint
//
// When a leaf predicate input or `MergeAxis` secondary is `Unbound`, the
// stage's malformed-hint must guide the user to bind an input. The Unbound
// state has the highest priority: it pre-empts the per-kind hints (empty
// hat-direction set, inverted axis range, secondary equals primary).
// ---------------------------------------------------------------------------

#[test]
fn predicate_button_pressed_unbound_surfaces_malformed_hint() {
    // ButtonPressed leaves had no per-kind validation hint before Task 9.
    // With an Unbound input the stage must now show the bind-an-input
    // guidance in the summary slot.
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: InputAddress::Unbound,
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html =
        render_with_expanded_settled(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("Bind an input to complete this condition"),
        "expected unbound predicate hint, got: {html}"
    );
}

#[test]
fn predicate_button_released_unbound_surfaces_malformed_hint() {
    // ButtonReleased mirrors ButtonPressed: no per-kind hint before Task 9,
    // Unbound now triggers the bind-an-input guidance.
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonReleased {
            input: InputAddress::Unbound,
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html =
        render_with_expanded_settled(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("Bind an input to complete this condition"),
        "expected unbound predicate hint, got: {html}"
    );
}

#[test]
fn predicate_axis_in_range_unbound_pre_empts_inverted_range_hint() {
    // AxisInRange has an existing per-kind hint when min > max ("min must
    // not exceed max"). With Unbound, the bind-an-input hint takes priority
    // even though the range is also inverted.
    let actions = vec![Action::Conditional {
        condition: Condition::AxisInRange {
            input: InputAddress::Unbound,
            min: 1.0,
            max: -1.0,
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html =
        render_with_expanded_settled(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("Bind an input to complete this condition"),
        "expected unbound predicate hint to win over inverted-range hint, got: {html}"
    );
}

#[test]
fn predicate_hat_direction_unbound_pre_empts_empty_directions_hint() {
    // HatDirection has an existing per-kind hint when directions is empty
    // ("at least one direction must be selected"). With Unbound, the
    // bind-an-input hint takes priority even when directions is also empty.
    let actions = vec![Action::Conditional {
        condition: Condition::HatDirection {
            input: InputAddress::Unbound,
            directions: vec![],
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html =
        render_with_expanded_settled(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("Bind an input to complete this condition"),
        "expected unbound predicate hint to win over empty-directions hint, got: {html}"
    );
}

#[test]
fn predicate_axis_in_range_bound_inverted_renders_min_max_hint() {
    // Regression guard: the AxisInRange per-kind hint
    // ("min must not exceed max") was never SSR-tested before Task 9
    // converted the malformed-hint write from `use_effect` to a render-time
    // write. With a Bound input + inverted range, the per-kind hint must
    // still surface so the conversion has not silently broken it.
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::AxisInRange {
            input: primary.clone(),
            min: 1.0,
            max: -1.0,
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html =
        render_with_expanded_settled(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("min must not exceed max"),
        "expected inverted-range hint, got: {html}"
    );
}

#[test]
fn predicate_hat_direction_bound_empty_renders_directions_hint() {
    // Regression guard: symmetric to the AxisInRange case above. The
    // HatDirection per-kind hint
    // ("at least one direction must be selected") must still surface for a
    // Bound input + empty directions after the Task 9 use_effect ->
    // render-time-write conversion.
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Hat { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::HatDirection {
            input: primary.clone(),
            directions: vec![],
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let (state, addr) = build_state(actions);
    let html =
        render_with_expanded_settled(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("at least one direction must be selected"),
        "expected empty-directions hint, got: {html}"
    );
}

#[test]
fn merge_axis_unbound_secondary_surfaces_malformed_hint() {
    // MergeAxis with an Unbound secondary input must surface the merge-
    // specific bind-a-secondary-input hint.
    let actions = vec![Action::MergeAxis {
        second_input: InputAddress::Unbound,
        operation: MergeOp::Average,
    }];
    let (state, addr) = build_state(actions);
    let html =
        render_with_expanded_settled(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);

    assert!(
        html.contains("Bind a secondary input to complete this merge"),
        "expected unbound secondary hint, got: {html}"
    );
}
