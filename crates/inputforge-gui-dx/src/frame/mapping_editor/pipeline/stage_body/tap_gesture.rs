// Rust guideline compliant 2026-05-14

//! `TapGesture` body: timing controls plus single/double-tap branches.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::{Action, ActionBranch};
use inputforge_core::engine::EngineCommand;

use crate::components::{InputSize, IntegerInput, Switch};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::pipeline::stage_body::branches::{BranchContainer, BranchSpec};
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::{
    dispatch_stage_edit, dispatch_stage_edit_into,
};
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, UndoKind, UndoLog, format_undo_label,
};

const THRESHOLD_MIN: usize = 1;
const THRESHOLD_MAX: usize = 10_000;

#[component]
pub(crate) fn TapGestureBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    threshold_ms: u64,
    fire_single_immediately: bool,
    single_tap: Vec<Action>,
    double_tap: Vec<Action>,
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let threshold_value = usize::try_from(threshold_ms).unwrap_or(THRESHOLD_MAX);
    let mut threshold_signal = use_signal(|| threshold_value);
    use_effect(use_reactive!(|threshold_value| {
        threshold_signal.set(threshold_value);
    }));

    let mut immediate_signal = use_signal(|| fire_single_immediately);
    use_effect(use_reactive!(|fire_single_immediately| {
        immediate_signal.set(fire_single_immediately);
    }));

    let current_name = ctx.config.read().mapping_names.get(&mapping_key.1).cloned();
    let threshold_id = format!(
        "if-stage-{}-tap-threshold",
        crate::frame::mapping_editor::pipeline::format_stage_id(&stage_id)
    );
    let switch_id = format!(
        "if-stage-{}-tap-immediate",
        crate::frame::mapping_editor::pipeline::format_stage_id(&stage_id)
    );

    let on_threshold_commit = {
        let mapping_key = mapping_key.clone();
        let stage_id = stage_id.clone();
        let root_actions = root_actions.clone();
        let cmd_tx = ctx.commands.clone();
        let mut undo_log = editor.undo_log;
        let current_name = current_name.clone();
        let single_tap = single_tap.clone();
        let double_tap = double_tap.clone();
        move |candidate: usize| {
            let current_threshold_ms = u64::try_from(threshold_signal()).unwrap_or(u64::MAX);
            let current_immediate = immediate_signal();
            let actions_before = root_actions_with_state(
                &root_actions,
                &stage_id,
                current_threshold_ms,
                current_immediate,
                single_tap.clone(),
                double_tap.clone(),
            );
            let new_threshold_ms = u64::try_from(candidate).unwrap_or(u64::MAX);
            threshold_signal.set(candidate);
            let label = edit_label(
                "threshold",
                format!("{current_threshold_ms} ms"),
                format!("{new_threshold_ms} ms"),
            );
            dispatch_stage_edit(
                &actions_before,
                &stage_id,
                Action::TapGesture {
                    threshold_ms: new_threshold_ms,
                    fire_single_immediately: current_immediate,
                    single_tap: single_tap.clone(),
                    double_tap: double_tap.clone(),
                },
                &mapping_key,
                current_name.clone(),
                &cmd_tx,
                &mut undo_log,
                label,
            );
        }
    };
    let on_toggle_change = {
        let mapping_key = mapping_key.clone();
        let stage_id = stage_id.clone();
        let root_actions = root_actions.clone();
        let cmd_tx = ctx.commands.clone();
        let mut undo_log = editor.undo_log;
        let current_name = current_name.clone();
        let single_tap = single_tap.clone();
        let double_tap = double_tap.clone();
        move |_evt: FormEvent| {
            let new_value = !immediate_signal();
            let current_threshold_ms = u64::try_from(threshold_signal()).unwrap_or(u64::MAX);
            let actions_before = root_actions_with_state(
                &root_actions,
                &stage_id,
                current_threshold_ms,
                immediate_signal(),
                single_tap.clone(),
                double_tap.clone(),
            );
            immediate_signal.set(new_value);
            let label = edit_label(
                "single timing",
                single_timing_label(!new_value),
                single_timing_label(new_value),
            );
            dispatch_stage_edit(
                &actions_before,
                &stage_id,
                Action::TapGesture {
                    threshold_ms: current_threshold_ms,
                    fire_single_immediately: new_value,
                    single_tap: single_tap.clone(),
                    double_tap: double_tap.clone(),
                },
                &mapping_key,
                current_name.clone(),
                &cmd_tx,
                &mut undo_log,
                label,
            );
        }
    };
    let branches = vec![
        BranchSpec {
            branch: ActionBranch::TapSingle,
            label: "Single tap",
            aria_label: "single tap branch",
            actions: single_tap.clone(),
        },
        BranchSpec {
            branch: ActionBranch::TapDouble,
            label: "Double tap",
            aria_label: "double tap branch",
            actions: double_tap.clone(),
        },
    ];

    rsx! {
        div { class: "if-stage__gesture-body",
            div { class: "if-stage__gesture-controls",
                label { class: "if-stage__gesture-control",
                    span { class: "if-stage__gesture-label", "Threshold" }
                    IntegerInput {
                        value: threshold_signal,
                        min: THRESHOLD_MIN,
                        max: THRESHOLD_MAX,
                        size: InputSize::Sm,
                        id: Some(threshold_id),
                        oncommit: on_threshold_commit,
                    }
                    span { class: "if-stage__gesture-unit", "ms" }
                }
                div { class: "if-stage__gesture-toggle",
                    Switch {
                        checked: immediate_signal,
                        id: Some(switch_id),
                        label: Some("Fire single tap immediately".to_owned()),
                        onchange: on_toggle_change,
                    }
                }
            }
            div { class: "if-stage__branches",
                BranchContainer {
                    mapping_key: mapping_key.clone(),
                    stage_id: stage_id.clone(),
                    specs: branches,
                    root_actions: root_actions.clone(),
                    depth,
                }
            }
        }
    }
}

pub(crate) fn dispatch_tap_gesture_edit_into(
    undo_log: &mut UndoLog,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
    label: String,
    threshold_ms: u64,
    fire_single_immediately: bool,
    single_tap: Vec<Action>,
    double_tap: Vec<Action>,
) {
    let new_action = Action::TapGesture {
        threshold_ms,
        fire_single_immediately,
        single_tap,
        double_tap,
    };
    dispatch_stage_edit_into(
        undo_log,
        root_actions,
        stage_id,
        new_action,
        mapping_key,
        name,
        cmd_tx,
        label,
    );
}

fn edit_label(field: &'static str, before: String, after: String) -> String {
    format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Tap gesture"),
            field: Some(field),
            before_after: Some((&before, &after)),
            ..LabelArgs::default()
        },
    )
}

fn single_timing_label(value: bool) -> String {
    if value {
        "immediate single".to_owned()
    } else {
        "exclusive single/double".to_owned()
    }
}

fn root_actions_with_state(
    root_actions: &[Action],
    stage_id: &StageId,
    threshold_ms: u64,
    fire_single_immediately: bool,
    single_tap: Vec<Action>,
    double_tap: Vec<Action>,
) -> Vec<Action> {
    replace_at_path(
        root_actions,
        stage_id,
        Action::TapGesture {
            threshold_ms,
            fire_single_immediately,
            single_tap,
            double_tap,
        },
    )
    .unwrap_or_else(|| root_actions.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::InputAddress;
    use std::sync::mpsc;

    #[test]
    fn toggle_after_local_threshold_edit_records_current_threshold_in_undo() {
        let mapping_key = ("Default".to_owned(), InputAddress::Unbound);
        let stage_id = StageId(vec![
            crate::frame::mapping_editor::undo_log::StageIdSegment::Index(0),
        ]);
        let stale_root_actions = vec![Action::TapGesture {
            threshold_ms: 250,
            fire_single_immediately: false,
            single_tap: Vec::new(),
            double_tap: Vec::new(),
        }];
        let actions_before_toggle = root_actions_with_state(
            &stale_root_actions,
            &stage_id,
            375,
            false,
            Vec::new(),
            Vec::new(),
        );
        let mut undo_log = UndoLog::default();
        let (tx, rx) = mpsc::channel();

        dispatch_tap_gesture_edit_into(
            &mut undo_log,
            &mapping_key,
            &stage_id,
            &actions_before_toggle,
            None,
            &tx,
            "Tap gesture: single timing exclusive single/double -> immediate single".to_owned(),
            375,
            true,
            Vec::new(),
            Vec::new(),
        );

        let command = rx
            .try_recv()
            .expect("toggle dispatches current local state");
        assert!(matches!(
            command,
            EngineCommand::SetMapping {
                actions,
                ..
            } if matches!(
                actions.first(),
                Some(Action::TapGesture {
                    threshold_ms: 375,
                    fire_single_immediately: true,
                    ..
                })
            )
        ));
        let before = &undo_log.stacks[&mapping_key].undo[0].mapping_before.actions[0];
        assert!(matches!(
            before,
            Action::TapGesture {
                threshold_ms: 375,
                fire_single_immediately: false,
                ..
            }
        ));
    }
}
