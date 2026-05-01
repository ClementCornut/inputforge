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

use inputforge_core::action::{Action, Condition};

use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::Pipeline;
use crate::frame::mapping_editor::pipeline::stage_body::predicate::PredicateEditor;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

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
    // Build the `path_prefix` for each branch by appending the branch segment
    // to the current stage_id's segments.
    let if_true_prefix: Vec<StageIdSegment> = {
        let mut p = stage_id.0.clone();
        p.push(StageIdSegment::IfTrue);
        p
    };
    let if_false_prefix: Vec<StageIdSegment> = {
        let mut p = stage_id.0.clone();
        p.push(StageIdSegment::IfFalse);
        p
    };

    // Nested pipelines receive `root_actions` UNCHANGED (Task 20 threading rule).
    let root_for_if_true = root_actions.clone();
    let root_for_if_false = root_actions.clone();
    let mapping_key_if_true = mapping_key.clone();
    let mapping_key_if_false = mapping_key.clone();

    // Child depth: one hop deeper than the current nesting.
    // Saturating add prevents overflow on pathological deep nesting.
    let child_depth = depth.saturating_add(1);

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

            div {
                class: "if-stage__branch",
                "aria-label": "if true branch",
                div { class: "if-stage__branch-label", "if true" }
                Pipeline {
                    mapping_key: mapping_key_if_true,
                    actions: if_true.clone(),
                    root_actions: root_for_if_true,
                    path_prefix: if_true_prefix,
                    depth: child_depth,
                }
            }

            div {
                class: "if-stage__branch",
                "aria-label": "if false branch",
                div { class: "if-stage__branch-label", "if false" }
                Pipeline {
                    mapping_key: mapping_key_if_false,
                    actions: if_false.clone(),
                    root_actions: root_for_if_false,
                    path_prefix: if_false_prefix,
                    depth: child_depth,
                }
            }
        }
    }
}
