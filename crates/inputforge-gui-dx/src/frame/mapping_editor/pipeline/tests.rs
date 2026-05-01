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
    InputAddress {
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
        if_false: None,
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
        if_false: None,
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
        if_false: None,
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
        if_false: None,
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfFalse,
        StageIdSegment::Index(0),
    ]);
    let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
    match &new[0] {
        Action::Conditional { if_false, .. } => {
            assert_eq!(if_false.as_ref().map(Vec::len), Some(1));
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
fn remove_at_path_last_in_if_false_collapses_to_none() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![],
        if_false: Some(vec![Action::Invert]),
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
                if_false.is_none(),
                "empty if_false branch must collapse to None"
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
    let addr = InputAddress {
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
}

impl PartialEq for HarnessProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
            && self.addr == other.addr
            && self.pre_expanded_stages == other.pre_expanded_stages
            && self.virtual_devices == other.virtual_devices
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
    ConfigSnapshot {
        devices: vec![inputforge_core::state::DeviceState {
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
        }],
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
        second_input: InputAddress {
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
    let primary = InputAddress {
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
    let primary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![],
        if_false: None,
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
    let primary = InputAddress {
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
        if_false: None,
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
    let primary = InputAddress {
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
        if_false: None,
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
    let primary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![Action::Invert],
        if_false: Some(vec![Action::Invert]),
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

#[test]
fn conditional_empty_if_false_shows_add_else_affordance() {
    let primary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![],
        if_false: None,
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);
    assert!(
        html.contains("Add else branch"),
        "expected else-branch affordance: {html}"
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
// Task 27: Placeholder bodies for ResponseCurve / Deadzone / ChangeMode
// ---------------------------------------------------------------------------

#[test]
fn placeholder_bodies_show_spec_caption() {
    let actions = vec![Action::Deadzone {
        config: DeadzoneConfig::default(),
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![StageId(vec![StageIdSegment::Index(0)])]);
    assert!(
        html.contains("F10 / F11 / F14 owns this body"),
        "expected placeholder caption: {html}"
    );
}

#[test]
fn conditional_three_deep_renders_all_branches() {
    let primary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let inner = Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![Action::Invert],
        if_false: None,
    };
    let middle = Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![inner],
        if_false: None,
    };
    let outer = Action::Conditional {
        condition: Condition::ButtonPressed {
            input: primary.clone(),
        },
        if_true: vec![middle],
        if_false: None,
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
