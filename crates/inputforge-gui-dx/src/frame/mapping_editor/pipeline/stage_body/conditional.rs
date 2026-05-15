// Rust guideline compliant 2026-05-02

//! `Conditional` body: predicate editor + two recursive branch sub-pipelines.
//!
//! # Structure
//!
//! ```text
//! ConditionalBody
//!   PredicateEditor        <- Task 26b
//!   if-true branch         <- Pipeline (root_actions UNCHANGED)
//!   if-false branch        <- Pipeline (root_actions UNCHANGED)
//! ```
//!
//! Both branches are always present; the false branch may be empty (encoded
//! as `Vec::new()`). The legacy "Add else branch" affordance was removed
//! 2026-05-02 along with the `Option<Vec<Action>>` indirection on the data
//! model; an empty pipeline now renders the standard `+ Add first stage`
//! placeholder, which is the same affordance used everywhere else.
//!
//! # Threading rule (Task 20)
//!
//! Both nested `Pipeline` mounts receive `root_actions` UNCHANGED (the
//! mapping's outermost actions vec). `StageId` paths are root-relative, so
//! all tree mutators (`replace_at_path`, `insert_at_path`, `remove_at_path`)
//! must be called against the root. The local branch slice is used for
//! rendering only.

use dioxus::prelude::*;

use inputforge_core::action::{Action, ActionBranch, Condition};

use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage_body::branches::{BranchContainer, BranchSpec};
use crate::frame::mapping_editor::pipeline::stage_body::predicate::PredicateEditor;
use crate::frame::mapping_editor::undo_log::StageId;

/// `Conditional` body component.
///
/// Renders the predicate editor, the `if_true` branch, and the `if_false`
/// branch. Both branches are always rendered as a nested `Pipeline`; an
/// empty `if_false` shows the same `+ Add first stage` placeholder as any
/// other empty pipeline.
#[component]
pub(crate) fn ConditionalBody(
    /// `(mode, InputAddress)` key for the mapping being edited.
    mapping_key: MappingKey,
    /// `StageId` of this `Conditional` stage (root-relative).
    stage_id: StageId,
    /// The predicate condition.
    condition: Condition,
    /// Actions in the `if_true` branch (local rendering slice only).
    if_true: Vec<Action>,
    /// Actions in the `if_false` branch (local rendering slice only).
    /// Empty vec encodes "do nothing when condition is false".
    if_false: Vec<Action>,
    /// Full root-level actions vec. Threaded UNCHANGED into both nested
    /// `Pipeline` mounts. All tree mutators operate on root-relative paths.
    root_actions: Vec<Action>,
    /// Nesting depth (0 = outer pipeline). Used by nested `Pipeline` to
    /// indent child stages correctly.
    depth: u8,
) -> Element {
    let branches = vec![
        BranchSpec {
            branch: ActionBranch::ConditionalTrue,
            label: "if true",
            aria_label: "if true branch",
            actions: if_true.clone(),
        },
        BranchSpec {
            branch: ActionBranch::ConditionalFalse,
            label: "if false",
            aria_label: "if false branch",
            actions: if_false.clone(),
        },
    ];

    rsx! {
        div { class: "if-stage__conditional-body",
            PredicateEditor {
                mapping_key: mapping_key.clone(),
                stage_id: stage_id.clone(),
                condition: condition.clone(),
                if_true: if_true.clone(),
                if_false: if_false.clone(),
                root_actions: root_actions.clone(),
            }

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
