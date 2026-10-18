// Rust guideline compliant 2026-05-14

//! Shared body for `TapGesture` and `PressGesture` stages.
//!
//! Both gesture variants render the same control surface (threshold input,
//! single-bool toggle, two named branches). The only differences are the
//! action variant they construct, the four label strings, and the two
//! `ActionBranch` enum tags. `GestureKind` carries those differences so a
//! single component handles both.

use dioxus::prelude::*;

use inputforge_core::action::Action;
#[path = "gesture_kind.rs"]
mod kind;
pub(crate) use kind::GestureKind;

use crate::components::{InputSize, IntegerInput, Switch};
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage_body::branches::{BranchContainer, BranchSpec};
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_field;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

const THRESHOLD_MIN: usize = 1;
const THRESHOLD_MAX: usize = 10_000;

#[component]
pub(crate) fn GestureBody(
    kind: GestureKind,
    mapping_key: MappingKey,
    stage_id: StageId,
    threshold_ms: u64,
    toggle: bool,
    branch_a_actions: Vec<Action>,
    branch_b_actions: Vec<Action>,
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    let threshold_value = usize::try_from(threshold_ms).unwrap_or(THRESHOLD_MAX);
    let mut threshold_signal = use_signal(|| threshold_value);
    use_effect(use_reactive!(|threshold_value| {
        threshold_signal.set(threshold_value);
    }));

    let mut toggle_signal = use_signal(|| toggle);
    use_effect(use_reactive!(|toggle| {
        toggle_signal.set(toggle);
    }));

    let formatted_stage_id = crate::frame::mapping_editor::pipeline::format_stage_id(&stage_id);
    let threshold_id = format!(
        "if-stage-{}-{}",
        formatted_stage_id,
        kind.threshold_id_suffix()
    );
    let switch_id = format!(
        "if-stage-{}-{}",
        formatted_stage_id,
        kind.toggle_id_suffix()
    );

    let on_threshold_commit = {
        let mapping_key = mapping_key.clone();
        let stage_id = stage_id.clone();
        let root_actions = root_actions.clone();
        move |candidate: usize| {
            let current_threshold_ms = u64::try_from(threshold_signal()).unwrap_or(u64::MAX);
            let new_threshold_ms = u64::try_from(candidate).unwrap_or(u64::MAX);
            let label = edit_label(
                kind,
                "threshold",
                &format!("{current_threshold_ms} ms"),
                &format!("{new_threshold_ms} ms"),
            );
            if dispatch_field(
                &root_actions,
                &stage_id,
                crate::mapping_edits::Field::GestureThreshold(candidate.to_string()),
                &mapping_key,
                label,
            ) {
                threshold_signal.set(candidate);
            }
        }
    };

    let on_toggle_change = {
        let mapping_key = mapping_key.clone();
        let stage_id = stage_id.clone();
        let root_actions = root_actions.clone();
        move |_: FormEvent| {
            let new_value = !toggle_signal();
            let label = edit_label(
                kind,
                kind.edit_field_label(),
                &kind.timing_label(!new_value),
                &kind.timing_label(new_value),
            );
            if dispatch_field(
                &root_actions,
                &stage_id,
                crate::mapping_edits::Field::GestureToggle(new_value),
                &mapping_key,
                label,
            ) {
                toggle_signal.set(new_value);
            }
        }
    };

    let (a_label, a_aria) = kind.branch_a_label();
    let (b_label, b_aria) = kind.branch_b_label();
    let branches = vec![
        BranchSpec {
            branch: kind.branch_a(),
            label: a_label,
            aria_label: a_aria,
            actions: branch_a_actions.clone(),
        },
        BranchSpec {
            branch: kind.branch_b(),
            label: b_label,
            aria_label: b_aria,
            actions: branch_b_actions.clone(),
        },
    ];

    rsx! {
        div { class: "if-stage__gesture-body",
            div { class: "if-stage__gesture-controls",
                crate::components::Label {
                    class: "if-stage__gesture-control",
                    for_id: threshold_id.clone(),
                    span { "Threshold" }
                    IntegerInput {
                        value: threshold_signal,
                        min: THRESHOLD_MIN,
                        max: THRESHOLD_MAX,
                        size: InputSize::Sm,
                        id: Some(threshold_id),
                        oncommit: on_threshold_commit,
                    }
                    span { "ms" }
                }
                div { class: "if-stage__gesture-toggle",
                    Switch {
                        checked: toggle_signal,
                        id: Some(switch_id),
                        label: Some(kind.toggle_label().to_owned()),
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

fn edit_label(kind: GestureKind, field: &'static str, before: &str, after: &str) -> String {
    format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some(kind.stage_name()),
            field: Some(field),
            before_after: Some((before, after)),
            ..LabelArgs::default()
        },
    )
}
