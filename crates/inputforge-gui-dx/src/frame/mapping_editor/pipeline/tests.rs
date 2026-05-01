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
    OutputAddress, OutputId, VJoyAxis,
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
}

impl PartialEq for HarnessProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
            && self.addr == other.addr
            && self.pre_expanded_stages == other.pre_expanded_stages
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
    } = props;

    let (cmd_tx, _) = mpsc::channel();
    let raw = RawHandles {
        state,
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
    };
    use_context_provider(|| raw.clone());

    let selection = ("Default".to_owned(), addr.clone());
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
    let mut vdom = VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            pre_expanded_stages,
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
