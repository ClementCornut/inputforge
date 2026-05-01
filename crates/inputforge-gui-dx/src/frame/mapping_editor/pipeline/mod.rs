// Rust guideline compliant 2026-05-01

//! F9 pipeline graph component tree.
//!
//! Composition (inside-out, in dependency order):
//!   - `at_path` / `replace_at_path` / `insert_at_path` / `remove_at_path`,
//!     pure `StageId` tree mutators used by every body and the `DnD` handler
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
            StageIdSegment::IfTrue => "T".to_owned(),
            StageIdSegment::IfFalse => "F".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

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
            StageIdSegment::IfTrue => match peek? {
                Action::Conditional { if_true, .. } => cursor = if_true.as_slice(),
                _ => return None,
            },
            StageIdSegment::IfFalse => match peek? {
                Action::Conditional { if_false, .. } => match if_false.as_deref() {
                    Some(branch) => cursor = branch,
                    None => return None,
                },
                _ => return None,
            },
        }
    }
    None
}

/// Replace the action at `path` with `replacement` and return the new tree.
///
/// Returns `None` for invalid paths (out-of-range index, missing branch,
/// expected `Conditional` got something else, empty path, path starting
/// with a branch segment). Callers must skip the edit and skip the
/// `push_edit` on `None`, otherwise a phantom undo entry would be created
/// against unchanged state. See `EditorState` mutator pattern in Task 22+.
#[must_use]
pub(crate) fn replace_at_path(
    actions: &[Action],
    path: &StageId,
    replacement: Action,
) -> Option<Vec<Action>> {
    fn walk(
        actions: &[Action],
        path: &[StageIdSegment],
        replacement: Action,
    ) -> Option<Vec<Action>> {
        let mut out = actions.to_vec();
        let (head, tail) = path.split_first()?;
        match head {
            StageIdSegment::Index(i) => {
                if tail.is_empty() {
                    if *i >= out.len() {
                        return None;
                    }
                    out[*i] = replacement;
                    Some(out)
                } else {
                    let target = out.get_mut(*i)?;
                    let (branch_seg, rest) = tail.split_first()?;
                    let Action::Conditional {
                        if_true, if_false, ..
                    } = target
                    else {
                        return None;
                    };
                    match branch_seg {
                        StageIdSegment::IfTrue => {
                            let new = walk(if_true.as_slice(), rest, replacement)?;
                            *if_true = new;
                        }
                        StageIdSegment::IfFalse => {
                            let current = if_false.clone()?;
                            let new = walk(&current, rest, replacement)?;
                            *if_false = if new.is_empty() { None } else { Some(new) };
                        }
                        StageIdSegment::Index(_) => return None,
                    }
                    Some(out)
                }
            }
            // StageId must always start with an Index segment.
            StageIdSegment::IfTrue | StageIdSegment::IfFalse => None,
        }
    }
    walk(actions, &path.0, replacement)
}

/// Insert `new_action` at `path`. The terminal segment must be an
/// `Index` indicating the insertion point; existing actions at that
/// index and beyond shift right. Indexes past the end append.
///
/// Returns `None` for invalid paths (empty, starts with branch, branch
/// segment after non-`Conditional`). Callers MUST skip the edit and skip
/// `push_edit` on `None` to avoid phantom undo entries.
///
/// # Structural-mutation invariant
///
/// When called from an `EditorState` mutator the caller MUST clear
/// `editor_state.expanded_stages` and `editor_state.malformed_hints`
/// AFTER dispatching the new actions. `StageId` paths are positional;
/// inserting a stage invalidates every cached path at or after the
/// mutation point. (Enforced by a test in Task 22.)
#[must_use]
pub(crate) fn insert_at_path(
    actions: &[Action],
    path: &StageId,
    new_action: Action,
) -> Option<Vec<Action>> {
    fn walk(
        actions: &[Action],
        path: &[StageIdSegment],
        new_action: Action,
    ) -> Option<Vec<Action>> {
        let mut out = actions.to_vec();
        let (head, tail) = path.split_first()?;
        match head {
            StageIdSegment::Index(i) => {
                if tail.is_empty() {
                    let pos = (*i).min(out.len());
                    out.insert(pos, new_action);
                    Some(out)
                } else {
                    let target = out.get_mut(*i)?;
                    let Action::Conditional {
                        if_true, if_false, ..
                    } = target
                    else {
                        return None;
                    };
                    let (branch_seg, rest) = tail.split_first()?;
                    match branch_seg {
                        StageIdSegment::IfTrue => {
                            *if_true = walk(if_true.as_slice(), rest, new_action)?;
                        }
                        StageIdSegment::IfFalse => {
                            let current = if_false.clone().unwrap_or_default();
                            let new = walk(&current, rest, new_action)?;
                            *if_false = if new.is_empty() { None } else { Some(new) };
                        }
                        StageIdSegment::Index(_) => return None,
                    }
                    Some(out)
                }
            }
            // StageId must always start with an Index segment.
            StageIdSegment::IfTrue | StageIdSegment::IfFalse => None,
        }
    }
    walk(actions, &path.0, new_action)
}

/// Remove the action at `path`. If the removal empties an `if_false`
/// branch, the branch collapses back to `None` (the engine's
/// "do nothing" shape). `if_true` always stays as a `Vec`, possibly
/// empty.
///
/// Returns `None` for invalid paths (empty, starts with branch,
/// out-of-range terminal index, branch segment after non-`Conditional`).
/// Callers MUST skip the edit and skip `push_edit` on `None` to avoid
/// phantom undo entries.
///
/// # Structural-mutation invariant
///
/// When called from an `EditorState` mutator the caller MUST clear
/// `editor_state.expanded_stages` and `editor_state.malformed_hints`
/// AFTER dispatching the new actions. `StageId` paths are positional;
/// removing a stage invalidates every cached path at or after the
/// mutation point. (Enforced by a test in Task 22.)
#[must_use]
pub(crate) fn remove_at_path(actions: &[Action], path: &StageId) -> Option<Vec<Action>> {
    fn walk(actions: &[Action], path: &[StageIdSegment]) -> Option<Vec<Action>> {
        let mut out = actions.to_vec();
        let (head, tail) = path.split_first()?;
        match head {
            StageIdSegment::Index(i) => {
                if tail.is_empty() {
                    if *i >= out.len() {
                        return None;
                    }
                    out.remove(*i);
                    Some(out)
                } else {
                    let target = out.get_mut(*i)?;
                    let Action::Conditional {
                        if_true, if_false, ..
                    } = target
                    else {
                        return None;
                    };
                    let (branch_seg, rest) = tail.split_first()?;
                    match branch_seg {
                        StageIdSegment::IfTrue => {
                            *if_true = walk(if_true.as_slice(), rest)?;
                        }
                        StageIdSegment::IfFalse => {
                            let branch = if_false.as_ref()?;
                            let new = walk(branch, rest)?;
                            *if_false = if new.is_empty() { None } else { Some(new) };
                        }
                        StageIdSegment::Index(_) => return None,
                    }
                    Some(out)
                }
            }
            // StageId must always start with an Index segment.
            StageIdSegment::IfTrue | StageIdSegment::IfFalse => None,
        }
    }
    walk(actions, &path.0)
}

/// Recursive pipeline component. Renders the action vector as `<ol>` of
/// `<Stage>` cards.
///
/// `mapping_key` identifies the mapping; `path_prefix` is the `StageId` path
/// that gets prepended to each stage's per-step `Index(i)` segment so nested
/// pipelines (Conditional branches) report deep IDs correctly.
///
/// `root_actions` is the mapping's outermost actions vec, threaded unchanged
/// through every recursion into Conditional branches. Bodies use it (NOT
/// `actions`) for `replace_at_path` / `insert_at_path` / `remove_at_path`
/// because `StageId` paths are root-relative. See the task description for the
/// recursion-correctness rationale.
#[component]
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
    if actions.is_empty() {
        return rsx! {
            div { class: "if-pipeline if-pipeline--empty",
                AddPalette {
                    mapping_key: mapping_key.clone(),
                    path_prefix: path_prefix.clone(),
                    target_len: 0,
                    root_actions: root_actions.clone(),
                    louder: true,
                }
            }
        };
    }

    // The parent pipeline path for the sortable group is the path_prefix
    // interpreted as a StageId. Each Stage uses this as its sortable group
    // discriminator so cross-pipeline drops are rejected by the validator.
    let parent_pipeline_path = StageId(path_prefix.clone());
    let path_prefix_for_iter = path_prefix.clone();
    let key_for_iter = mapping_key.clone();
    let root_for_iter = root_actions.clone();
    let actions_len = actions.len();

    rsx! {
        ol { class: "if-pipeline",
            for (i, action) in actions.iter().enumerate() {
                {
                    let mut path = path_prefix_for_iter.clone();
                    path.push(StageIdSegment::Index(i));
                    let stage_id = StageId(path);
                    rsx! {
                        Stage {
                            key: "{i}",
                            stage_id,
                            mapping_key: key_for_iter.clone(),
                            action: action.clone(),
                            root_actions: root_for_iter.clone(),
                            parent_pipeline_path: parent_pipeline_path.clone(),
                            depth,
                        }
                    }
                }
            }
            li { class: "if-pipeline__add-end",
                AddPalette {
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
