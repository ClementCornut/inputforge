// Rust guideline compliant 2026-05-14

//! Shared body for `TapGesture` and `PressGesture` stages.
//!
//! Both gesture variants render the same control surface (threshold input,
//! single-bool toggle, two named branches). The only differences are the
//! action variant they construct, the four label strings, and the two
//! `ActionBranch` enum tags. `GestureKind` carries those differences so a
//! single component handles both.

use dioxus::prelude::*;

use inputforge_core::action::{Action, ActionBranch};

use crate::components::{InputSize, IntegerInput, Switch};
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage_body::branches::{BranchContainer, BranchSpec};
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_field;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

const THRESHOLD_MIN: usize = 1;
const THRESHOLD_MAX: usize = 10_000;

/// Discriminator carrying everything that differs between `TapGesture` and
/// `PressGesture` rendering: action constructor, IDs, labels, branch tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GestureKind {
    Tap,
    Press,
}

impl GestureKind {
    fn stage_name(self) -> &'static str {
        match self {
            Self::Tap => "Tap gesture",
            Self::Press => "Press gesture",
        }
    }

    fn threshold_id_suffix(self) -> &'static str {
        match self {
            Self::Tap => "tap-threshold",
            Self::Press => "press-threshold",
        }
    }

    fn toggle_id_suffix(self) -> &'static str {
        match self {
            Self::Tap => "tap-immediate",
            Self::Press => "press-long-fire",
        }
    }

    fn toggle_label(self) -> &'static str {
        match self {
            Self::Tap => "Fire single tap immediately",
            Self::Press => "Fire long press when threshold is crossed",
        }
    }

    fn edit_field_label(self) -> &'static str {
        match self {
            Self::Tap => "single timing",
            Self::Press => "long timing",
        }
    }

    fn timing_label(self, value: bool) -> String {
        match (self, value) {
            (Self::Tap, true) => "immediate single".to_owned(),
            (Self::Tap, false) => "exclusive single/double".to_owned(),
            (Self::Press, true) => "long fires while held".to_owned(),
            (Self::Press, false) => "decide on release".to_owned(),
        }
    }

    fn branch_a(self) -> ActionBranch {
        match self {
            Self::Tap => ActionBranch::TapSingle,
            Self::Press => ActionBranch::PressShort,
        }
    }

    fn branch_b(self) -> ActionBranch {
        match self {
            Self::Tap => ActionBranch::TapDouble,
            Self::Press => ActionBranch::PressLong,
        }
    }

    fn branch_a_label(self) -> (&'static str, &'static str) {
        match self {
            Self::Tap => ("Single tap", "single tap branch"),
            Self::Press => ("Short press", "short press branch"),
        }
    }

    fn branch_b_label(self) -> (&'static str, &'static str) {
        match self {
            Self::Tap => ("Double tap", "double tap branch"),
            Self::Press => ("Long press", "long press branch"),
        }
    }
}

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
