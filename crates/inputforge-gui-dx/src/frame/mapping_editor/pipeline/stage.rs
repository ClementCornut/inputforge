// Rust guideline compliant 2026-05-01

//! Stage card: header + body container.
//!
//! Renders one action as a collapsible card. Category tint is applied via
//! BEM modifier classes (`is-processing`, `is-output`, `is-control`).
//! The body region is a placeholder until Task 22 wires the dispatcher.

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body;
use crate::frame::mapping_editor::pipeline::stage_header::StageHeader;
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn Stage(
    stage_id: StageId,
    /// `(mode, InputAddress)` key for the mapping being edited. Named
    /// `mapping_key` to avoid collision with Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    action: Action,
    /// Mapping's root actions vec, threaded unchanged through every
    /// recursion. Bodies use this for tree mutators because `StageId`
    /// paths are root-relative. See `Pipeline` doc for rationale.
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    let editor = use_context::<EditorState>();
    let expanded = editor.expanded_stages.read().contains(&stage_id);

    let category_class = match &action {
        Action::ResponseCurve { .. } | Action::Deadzone { .. } | Action::Invert => "is-processing",
        Action::MapToVJoy { .. } | Action::MapToKeyboard { .. } | Action::MergeAxis { .. } => {
            "is-output"
        }
        Action::ChangeMode { .. } | Action::Conditional { .. } => "is-control",
    };

    let class = format!("if-stage {category_class}");
    let title = stage_title(&action);
    let summary = stage_summary(&action);
    let right_slot = stage_body::header_right_slot(&action, expanded);
    let body_id = format!(
        "if-stage-body-{}",
        stage_id
            .0
            .iter()
            .map(|seg| {
                use crate::frame::mapping_editor::undo_log::StageIdSegment;
                match seg {
                    StageIdSegment::Index(i) => format!("{i}"),
                    StageIdSegment::IfTrue => "T".to_owned(),
                    StageIdSegment::IfFalse => "F".to_owned(),
                }
            })
            .collect::<Vec<_>>()
            .join(".")
    );

    rsx! {
        li {
            class: "{class}",
            "data-stage-id": "{format_stage_id(&stage_id)}",
            StageHeader {
                stage_id: stage_id.clone(),
                title,
                summary,
                expanded,
                right_slot,
            }
            if expanded {
                div {
                    id: "{body_id}",
                    class: "if-stage__body",
                    // Body dispatcher lands in Task 22.
                    div { class: "if-stage__body-placeholder", "(body)" }
                }
            }
        }
    }
}

pub(crate) fn stage_title(action: &Action) -> String {
    match action {
        Action::Invert => "Invert".to_owned(),
        Action::Deadzone { .. } => "Deadzone".to_owned(),
        Action::ResponseCurve { .. } => "Response curve".to_owned(),
        Action::MapToVJoy { .. } => "Map to vJoy".to_owned(),
        Action::MapToKeyboard { .. } => "Map to keyboard".to_owned(),
        Action::MergeAxis { .. } => "Merge axis".to_owned(),
        Action::ChangeMode { .. } => "Change mode".to_owned(),
        Action::Conditional { .. } => "Conditional".to_owned(),
    }
}

pub(crate) fn stage_summary(_action: &Action) -> String {
    // Per-variant summaries land in Task 21; this stub keeps the header
    // layout stable in the meantime.
    String::new()
}

pub(crate) fn format_stage_id(id: &StageId) -> String {
    use crate::frame::mapping_editor::undo_log::StageIdSegment;
    id.0.iter()
        .map(|seg| match seg {
            StageIdSegment::Index(i) => format!("{i}"),
            StageIdSegment::IfTrue => "T".to_owned(),
            StageIdSegment::IfFalse => "F".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Suppress unused-variable warning for `depth` and `mapping_key` until
/// Task 26a and Task 22 consume them respectively.
const _: () = {
    fn _assert_depth_used(_d: u8) {}
    fn _assert_mk_used(_k: &MappingKey) {}
};
