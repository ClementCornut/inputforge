// Rust guideline compliant 2026-05-14

//! Shared branch rendering for control-flow stage bodies.

use dioxus::prelude::*;

use inputforge_core::action::{Action, ActionBranch};

use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::Pipeline;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BranchSpec {
    pub(crate) branch: ActionBranch,
    pub(crate) label: &'static str,
    pub(crate) aria_label: &'static str,
    pub(crate) actions: Vec<Action>,
}

#[component]
pub(crate) fn BranchContainer(
    mapping_key: MappingKey,
    stage_id: StageId,
    specs: Vec<BranchSpec>,
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    let child_depth = depth.saturating_add(1);

    rsx! {
        for spec in specs {
            div {
                class: "if-stage__branch",
                "aria-label": spec.aria_label,
                div { class: "if-stage__branch-label", "{spec.label}" }
                Pipeline {
                    mapping_key: mapping_key.clone(),
                    actions: spec.actions.clone(),
                    root_actions: root_actions.clone(),
                    path_prefix: branch_path(&stage_id, spec.branch),
                    depth: child_depth,
                }
            }
        }
    }
}

fn branch_path(stage_id: &StageId, branch: ActionBranch) -> Vec<StageIdSegment> {
    let mut path = stage_id.0.clone();
    path.push(StageIdSegment::Branch(branch));
    path
}
