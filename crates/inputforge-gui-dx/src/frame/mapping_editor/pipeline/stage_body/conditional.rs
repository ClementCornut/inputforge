// Rust guideline compliant 2026-05-01

//! `Conditional` body: predicate stub + two recursive branch sub-pipelines.
//!
//! # Structure
//!
//! ```text
//! ConditionalBody
//!   PredicateEditor        <- Task 26b replaces the stub
//!   if-true branch         <- Pipeline (root_actions UNCHANGED)
//!   if-false branch        <- Pipeline (root_actions UNCHANGED) OR "Add else branch" button
//! ```
//!
//! # Threading rule (Task 20)
//!
//! Both nested `Pipeline` mounts receive `root_actions` UNCHANGED (the
//! mapping's outermost actions vec). `StageId` paths are root-relative, so
//! all tree mutators (`replace_at_path`, `insert_at_path`, `remove_at_path`)
//! must be called against the root. The local branch slice is used for
//! rendering only.
//!
//! # Add-else-branch button
//!
//! When `if_false` is `None`, a dashed affordance button is rendered instead
//! of a nested pipeline. Clicking it dispatches `SetMapping` with
//! `if_false: Some(vec![])`, adds a undo entry, and clears
//! `expanded_stages` and `malformed_hints` per Task 11's structural-mutation
//! invariant.
//!
//! # Amendments applied
//!
//! 1. Name preservation: current name read from `cfg.mapping_names`.
//! 2. Snapshot-before-dispatch: `before` Mapping built before dispatch.
//! 3. Dispatch-before-undo: `push_edit` called only after `cmd.send` succeeds.
//! 4. Structural-mutation cleanup: `expanded_stages` and `malformed_hints`
//!    cleared after an add-else-branch structural mutation (Task 11).
//! 5. External-edit subscription: `use_effect` subscribes to
//!    `editor.external_edit_reset` to participate in the reactive graph.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition, Mapping};
use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body::predicate::PredicateEditor;
use crate::frame::mapping_editor::pipeline::{Pipeline, replace_at_path};
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, StageIdSegment, UndoKind, format_undo_label,
};

/// `Conditional` body component.
///
/// Renders the predicate stub, the `if_true` branch as a nested `Pipeline`,
/// and either a nested `Pipeline` for `if_false` (when `Some`) or an
/// "Add else branch" dashed affordance button (when `None`).
#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event \
              listener attribute shorthand (onclick: on_add_else)."
)]
pub(crate) fn ConditionalBody(
    /// `(mode, InputAddress)` key for the mapping being edited.
    mapping_key: MappingKey,
    /// `StageId` of this `Conditional` stage (root-relative).
    stage_id: StageId,
    /// The predicate condition.
    condition: Condition,
    /// Actions in the `if_true` branch (local rendering slice only).
    if_true: Vec<Action>,
    /// Actions in the `if_false` branch, or `None` when the else branch
    /// has not been added yet.
    if_false: Option<Vec<Action>>,
    /// Full root-level actions vec. Threaded UNCHANGED into both nested
    /// `Pipeline` mounts and into the add-else-branch dispatcher. All tree
    /// mutators operate on root-relative paths.
    root_actions: Vec<Action>,
    /// Nesting depth (0 = outer pipeline). Used by nested `Pipeline` to
    /// indent child stages correctly.
    depth: u8,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();

    // Amendment 5: subscribe to external_edit_reset so Dioxus includes this
    // component in the reactive graph. No local Signals to reset here; the
    // subscription ensures re-renders when Task 33 advances the token.
    let reset_token = editor.external_edit_reset;
    use_effect(move || {
        let _ = *reset_token.read();
    });

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

    // Clone props needed in the add-else-branch click handler.
    let key_for_add = mapping_key.clone();
    let stage_id_for_add = stage_id.clone();
    let root_for_add = root_actions.clone();
    let condition_for_add = condition.clone();
    let if_true_for_add = if_true.clone();
    let cmd_tx_add = ctx.commands.clone();
    let mut undo_log_add = editor.undo_log;
    let mut expanded_stages_add = editor.expanded_stages;
    let mut malformed_hints_add = editor.malformed_hints;

    // --- Add-else-branch click handler ---
    //
    // Builds a replacement Conditional with `if_false: Some(vec![])`,
    // dispatches SetMapping, and on success clears positional caches and
    // pushes an undo entry.
    let on_add_else = move |_: MouseEvent| {
        // Amendment 1: preserve user-set mapping name.
        let cfg = ctx.config.read();
        let current_name = cfg.mapping_names.get(&key_for_add.1).cloned();
        drop(cfg);

        let new_action = Action::Conditional {
            condition: condition_for_add.clone(),
            if_true: if_true_for_add.clone(),
            // Structural mutation: initialise else branch as an empty vec.
            if_false: Some(vec![]),
        };

        let Some(new_actions) = replace_at_path(&root_for_add, &stage_id_for_add, new_action)
        else {
            // Invalid path: skip to avoid phantom undo entry.
            return;
        };

        // Amendment 2: snapshot before dispatch.
        let before = Mapping {
            input: key_for_add.1.clone(),
            mode: key_for_add.0.clone(),
            name: current_name.clone(),
            actions: root_for_add.clone(),
        };

        // Amendment 3: dispatch before undo push.
        if cmd_tx_add
            .send(EngineCommand::SetMapping {
                input: key_for_add.1.clone(),
                mode: key_for_add.0.clone(),
                name: current_name,
                actions: new_actions,
            })
            .is_err()
        {
            tracing::warn!(
                target: "f9::mapping_editor",
                action = "conditional_add_else_drop_offline",
                "add-else-branch dropped: engine channel disconnected"
            );
            return;
        }

        // Amendment 4: structural-mutation invariant (Task 11). Clear both
        // caches so stale root-relative paths do not corrupt expand/hint state.
        expanded_stages_add.write().clear();
        malformed_hints_add.write().clear();

        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Conditional"),
                field: Some("else branch"),
                ..LabelArgs::default()
            },
        );
        undo_log_add
            .write()
            .push_edit(key_for_add.clone(), before, UndoKind::StageEdit, label);
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
            // Predicate stub; Task 26b replaces this with the real editor.
            PredicateEditor {
                mapping_key: mapping_key.clone(),
                stage_id: stage_id.clone(),
                condition: condition.clone(),
                if_true: if_true.clone(),
                if_false: if_false.clone(),
                root_actions: root_actions.clone(),
            }

            // if-true branch
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

            // if-false branch: nested Pipeline or Add affordance.
            if let Some(else_actions) = if_false.clone() {
                div {
                    class: "if-stage__branch",
                    "aria-label": "if false branch",
                    div { class: "if-stage__branch-label", "if false" }
                    Pipeline {
                        mapping_key: mapping_key_if_false,
                        actions: else_actions,
                        root_actions: root_for_if_false,
                        path_prefix: if_false_prefix,
                        depth: child_depth,
                    }
                }
            } else {
                button {
                    r#type: "button",
                    class: "if-stage__add-else-branch",
                    onclick: on_add_else,
                    "+ Add else branch"
                }
            }
        }
    }
}
