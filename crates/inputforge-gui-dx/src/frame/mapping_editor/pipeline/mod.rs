// Rust guideline compliant 2026-05-01

//! F9 pipeline graph component tree.
//!
//! Composition (inside-out, in dependency order):
//!   - `at_path`, prepared action lookup for body rendering and callback targets
//!   - `stage_body::*`, per-variant body components
//!   - `stage_header`, title + summary + chevron
//!   - `stage`, header + body container
//!   - `Pipeline`, ordered list orchestrator (recursive for Conditional)

#![allow(
    dead_code,
    reason = "submodules export APIs consumed across the editor; clippy's \
              reachability check loses some pub(crate) items here."
)]

mod add_palette;
pub(crate) mod dnd;
mod stage;
mod stage_actions_menu;
pub(crate) mod stage_body;
mod stage_header;

#[cfg(test)]
mod keyboard_events_tests;
#[cfg(test)]
mod pointer_events_tests;
#[cfg(test)]
mod tests;

use add_palette::AddPalette;
pub(crate) use stage::Stage;
pub(crate) use stage_actions_menu::StageActionsMenu;

/// Render a [`StageId`] as a dot-separated string of path segments.
///
/// Used by both `stage.rs` (for `data-stage-id` attributes) and
/// `stage_header.rs` (for `aria-controls` IDs). Centralised here so both
/// modules share one definition.
pub(super) fn format_stage_id(id: &StageId) -> String {
    id.0.iter()
        .map(|seg| match seg {
            StageIdSegment::Index(i) => format!("{i}"),
            StageIdSegment::Branch(branch) => match branch {
                ActionBranch::ConditionalTrue => "T".to_owned(),
                ActionBranch::ConditionalFalse => "F".to_owned(),
                ActionBranch::TapSingle => "TS".to_owned(),
                ActionBranch::TapDouble => "TD".to_owned(),
                ActionBranch::PressShort => "PS".to_owned(),
                ActionBranch::PressLong => "PL".to_owned(),
            },
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Returns `true` if `stage_id` is invalidated by a structural mutation
/// at index `mutation_index` inside `parent_path`'s branch.
///
/// A path is invalidated when it sits in the same branch as the mutation
/// AND its index at the mutation level is at-or-after `mutation_index`.
/// Strict-ancestor paths, paths in other branches, and earlier-sibling
/// paths are NOT invalidated and should be preserved across the mutation.
///
/// Used by every dispatch site that performs an insert / remove / move:
/// instead of clearing the entire `expanded_stages` and `malformed_hints`
/// caches (which collapses ancestors and unrelated siblings), retain only
/// the paths that survived the index shift. This is what keeps the parent
/// `Conditional` expanded after adding a stage to one of its branches.
pub(super) fn path_invalidated_by_mutation(
    stage_id: &StageId,
    parent_path: &[StageIdSegment],
    mutation_index: usize,
) -> bool {
    let parent_len = parent_path.len();
    if stage_id.0.len() <= parent_len {
        return false;
    }
    if stage_id.0[..parent_len] != *parent_path {
        return false;
    }
    match stage_id.0[parent_len] {
        StageIdSegment::Index(idx) => idx >= mutation_index,
        // Branch segment: different sub-branch from
        // the mutation, which targets an Index position. Preserve.
        StageIdSegment::Branch(_) => false,
    }
}

use dioxus::prelude::*;

use inputforge_core::action::{Action, ActionBranch, branch_actions};

use crate::components::sortable::{SortableGap, SortableState};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::dnd::{
    gap_to_post_remove_slot, validate_pipeline_drop,
};
use crate::frame::mapping_editor::pipeline::stage::stage_title_for;
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, StageIdSegment, UndoKind, format_undo_label,
};

/// Read the action at `path` in `actions`. Returns `None` when the
/// path does not resolve (out-of-range index, missing branch, etc.).
#[must_use]
pub(crate) fn at_path<'a>(actions: &'a [Action], path: &StageId) -> Option<&'a Action> {
    let mut cursor: &[Action] = actions;
    let mut peek: Option<&Action> = None;
    let mut iter = path.0.iter().peekable();
    while let Some(seg) = iter.next() {
        match seg {
            StageIdSegment::Index(i) => {
                let action = cursor.get(*i)?;
                if iter.peek().is_none() {
                    return Some(action);
                }
                peek = Some(action);
            }
            StageIdSegment::Branch(branch) => cursor = branch_actions(peek?, *branch)?,
        }
    }
    None
}

/// Recursive pipeline component. Renders the action vector as an `<ol>`
/// of `<Stage>` cards interleaved with `<SortableGap>` drop zones.
///
/// `mapping_key` identifies the mapping; `path_prefix` is the `StageId`
/// path that gets prepended to each stage's per-step `Index(i)` segment
/// so nested pipelines (Conditional branches) report deep IDs correctly.
///
/// `root_actions` is the mapping's outermost actions vec, threaded
/// unchanged through every recursion into Conditional branches. Bodies
/// use it (NOT `actions`) for `replace_at_path` / `insert_at_path` /
/// `remove_at_path` because `StageId` paths are root-relative.
///
/// Pipeline owns the drag-drop dispatch closure. Each `SortableGap` in
/// this pipeline shares one `EventHandler<usize>` (built once per
/// render, threaded into every gap as a `Copy` handle). The closure
/// reads source state (`drag_from`, `drag_group`) from the shared
/// `SortableState`, converts the `gap_index` to a post-remove insertion
/// slot, mutates the action tree via `remove_at_path` + `insert_at_
/// path`, dispatches `EngineCommand::SetMapping`, and pushes a
/// `StageReorder` undo entry.
#[component]
#[allow(
    clippy::too_many_lines,
    reason = "Pipeline owns the inline drop closure (the ex-stage.rs DnD body) so \
              it can be threaded into every gap as a single shared EventHandler."
)]
pub(crate) fn Pipeline(
    /// `(mode, InputAddress)` key for the mapping. Named `mapping_key` to
    /// avoid collision with Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    /// This pipeline's local action slice (used for rendering and `StageId`
    /// derivation only).
    actions: Vec<Action>,
    /// Mapping's outermost actions vec, threaded unchanged through every
    /// recursion. Bodies use this for tree mutators; local `actions` is
    /// rendering-only.
    root_actions: Vec<Action>,
    /// `StageId` prefix segments from ancestor pipelines. Empty at the outer
    /// mount; Conditional recursion (Task 26a) appends branch segments.
    path_prefix: Vec<StageIdSegment>,
    /// Indent depth (0 = outer pipeline; +1 per Conditional branch hop).
    depth: u8,
) -> Element {
    stage_body::instruments::stage_dispatch::use_origin();
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let sortable = use_context::<SortableState<StageId>>();
    let profile_name = ctx.meta.read().profile_name.clone();
    let drag_context = format!(
        "profile:{:?};mapping:{mapping_key:?};actions:{root_actions:?}",
        ctx.meta.read().profile_id
    );

    // The parent pipeline path for the sortable group is the
    // `path_prefix` interpreted as a `StageId`. Every gap in this
    // pipeline shares this group discriminator.
    let parent_pipeline_path = StageId(path_prefix.clone());

    // Build the per-pipeline drop handler. Each SortableGap below
    // clones this `EventHandler` (which is `Copy`) into its props.
    let mut expanded_stages = editor.expanded_stages;
    let mut malformed_hints = editor.malformed_hints;
    let mut malformed_summary_tags = editor.malformed_summary_tags;
    let drag_from_for_drop = sortable.drag_from;
    let drag_group_for_drop = sortable.drag_group;
    let mut live_writer = sortable.live_announcement;
    let parent_path_for_drop = parent_pipeline_path.clone();
    let mapping_key_for_drop = mapping_key.clone();
    let root_for_drop = root_actions.clone();

    // Bind the validator with an explicit type so Dioxus's prop SuperInto
    // accepts it as `Option<fn(&StageId, &StageId) -> bool>` (the
    // fn-item type of `validate_pipeline_drop` is unique and wouldn't
    // coerce through the macro's generated trait bound).
    let pipeline_validator: Option<fn(&StageId, &StageId) -> bool> = Some(validate_pipeline_drop);

    let drop_handler = EventHandler::new(move |gap_index: usize| {
        // `drag_from` holds the source's group-local index; still
        // populated when the closure runs (the gap clears it after we
        // return).
        let Some(src_local_index) = *drag_from_for_drop.peek() else {
            return;
        };
        // `drag_group` holds the source's parent pipeline path.
        let Some(src_parent_path) = drag_group_for_drop.peek().clone() else {
            return;
        };

        // Reconstruct the source's full StageId.
        let mut src_segs = src_parent_path.0.clone();
        src_segs.push(StageIdSegment::Index(src_local_index));
        let src_id = StageId(src_segs);

        // Convert the gap's pre-remove slot index to a post-remove
        // insertion index. See `gap_to_post_remove_slot` for the math.
        let same_branch = src_parent_path == parent_path_for_drop;
        let post_remove_to = gap_to_post_remove_slot(
            &src_parent_path,
            &parent_path_for_drop,
            src_local_index,
            gap_index,
        );

        // Reconstruct the target's full StageId in the post-remove tree.
        let mut tgt_segs = parent_path_for_drop.0.clone();
        tgt_segs.push(StageIdSegment::Index(post_remove_to));
        let tgt_id = StageId(tgt_segs);

        // Friendly stage name for the undo label, looked up before the
        // mutation so the original tree resolves.
        let stage_name = at_path(&root_for_drop, &src_id).map_or("stage", stage_title_for);
        let label = format_undo_label(
            UndoKind::StageReorder,
            LabelArgs {
                stage_name: Some(stage_name),
                from_to: Some((src_local_index, post_remove_to)),
                ..LabelArgs::default()
            },
        );
        if !stage_body::instruments::stage_dispatch::submit(
            &mapping_key_for_drop,
            &root_for_drop,
            crate::mapping_edits::Edit::Move {
                from: src_id,
                to: tgt_id,
            },
            label,
            None,
            true,
        ) {
            return;
        }

        // Drag-reorder shifts indices in the source branch (from
        // src_local_index) and in the target branch (from
        // post_remove_to). When src and target share a parent, the
        // affected range is [min(src, to), ...]. Invalidate only paths
        // in those affected ranges; ancestors and unrelated branches
        // keep their expanded state, so the parent Conditional / outer
        // pipeline does not collapse on a drop.
        let src_parent_segs = src_parent_path.0.clone();
        let tgt_parent_segs = parent_path_for_drop.0.clone();
        let invalidate_src_from = if same_branch {
            src_local_index.min(post_remove_to)
        } else {
            src_local_index
        };
        let invalidate_tgt_from = if same_branch {
            src_local_index.min(post_remove_to)
        } else {
            post_remove_to
        };
        let invalidated = |p: &StageId| {
            path_invalidated_by_mutation(p, &src_parent_segs, invalidate_src_from)
                || path_invalidated_by_mutation(p, &tgt_parent_segs, invalidate_tgt_from)
        };
        expanded_stages.write().retain(|p| !invalidated(p));
        malformed_hints.write().retain(|p, _| !invalidated(p));
        malformed_summary_tags
            .write()
            .retain(|p, _| !invalidated(p));

        // AT live-region announcement.
        live_writer.set(format!(
            "Stage moved from position {} to {}",
            src_local_index + 1,
            post_remove_to + 1
        ));
    });

    if actions.is_empty() {
        let add_palette_key = format!("{profile_name:?}:{mapping_key:?}:{path_prefix:?}:0");
        // Empty pipeline (e.g. an empty `if_false` branch). One gap
        // (gap_index 0) sits as a sibling of `AddPalette` so the user
        // can drop a stage from another pipeline into the empty branch.
        // Both share the `.if-pipeline.if-pipeline--empty` container so
        // the empty case stays structurally close to the non-empty case
        // (`<ol>` with `<li>` children).
        return rsx! {
            ol { class: "if-pipeline if-pipeline--empty", "data-sortable-group": "{sortable.drag_from.id():?}", "data-sortable-context": "{drag_context}",
                SortableGap {
                    state: sortable,
                    gap_index: 0_usize,
                    group: parent_pipeline_path.clone(),
                    validate_drop: pipeline_validator,
                    on_drop: drop_handler,
                }
                li { class: "if-pipeline__add-end",
                    AddPalette {
                        key: "{add_palette_key}",
                        mapping_key: mapping_key.clone(),
                        path_prefix: path_prefix.clone(),
                        target_len: 0,
                        root_actions: root_actions.clone(),
                        louder: true,
                    }
                }
            }
        };
    }

    let path_prefix_for_iter = path_prefix.clone();
    let key_for_iter = mapping_key.clone();
    let root_for_iter = root_actions.clone();
    let actions_len = actions.len();
    let add_palette_key = format!("{profile_name:?}:{mapping_key:?}:{path_prefix:?}:{actions_len}");
    let parent_path_for_gaps = parent_pipeline_path.clone();

    rsx! {
        ol { class: "if-pipeline", "data-sortable-group": "{sortable.drag_from.id():?}", "data-sortable-context": "{drag_context}",
            SortableGap {
                key: "gap-0",
                state: sortable,
                gap_index: 0_usize,
                group: parent_path_for_gaps.clone(),
                validate_drop: pipeline_validator,
                on_drop: drop_handler,
            }
            for (i, action) in actions.iter().enumerate() {
                {
                    let mut path = path_prefix_for_iter.clone();
                    path.push(StageIdSegment::Index(i));
                    let stage_id = StageId(path);
                    rsx! {
                        Stage {
                            drag_context: drag_context.clone(),
                            key: "stage-{i}",
                            stage_id,
                            mapping_key: key_for_iter.clone(),
                            action: action.clone(),
                            root_actions: root_for_iter.clone(),
                            parent_pipeline_path: parent_pipeline_path.clone(),
                            depth,
                        }
                        SortableGap {
                            state: sortable,
                            gap_index: i + 1,
                            group: parent_path_for_gaps.clone(),
                            validate_drop: pipeline_validator,
                            on_drop: drop_handler,
                        }
                    }
                }
            }
            li { class: "if-pipeline__add-end",
                AddPalette {
                    key: "{add_palette_key}",
                    mapping_key: mapping_key.clone(),
                    path_prefix: path_prefix.clone(),
                    target_len: actions_len,
                    root_actions: root_actions.clone(),
                    louder: false,
                }
            }
        }
    }
}
