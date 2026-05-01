// Rust guideline compliant 2026-05-01

//! Right-click stage actions menu (Move up / Move down / Delete).
//!
//! Anchored at cursor coordinates via a `position: fixed` wrapper because
//! the F2 `MenuRoot` does not expose anchor coordinates. See Task 29 for
//! the full design rationale.
//!
//! # Structural-mutation contract
//!
//! Move up/down and Delete both clear `EditorState::expanded_stages` and
//! `EditorState::malformed_hints` after a successful dispatch. `StageId`
//! paths are positional; swapping or removing a stage invalidates every
//! cached path at or after the mutation point. (Per Task 11 invariant.)

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::stage::stage_title_for;
use crate::frame::mapping_editor::pipeline::{remove_at_path, replace_at_path};
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, StageIdSegment, UndoKind, format_undo_label,
};
use crate::frame::mapping_editor::{EditorState, StageMenuState};

// ---------------------------------------------------------------------------
// Private path helpers
// ---------------------------------------------------------------------------

/// Split a `StageId` into the parent path (all segments except the last)
/// and the final `Index` value.
///
/// Returns `None` when the path is empty or the last segment is not an
/// `Index` (which would indicate a malformed caller).
fn split_stage_path(id: &StageId) -> Option<(Vec<StageIdSegment>, usize)> {
    let segs = &id.0;
    let last = segs.last()?;
    let StageIdSegment::Index(idx) = *last else {
        return None;
    };
    let parent = segs[..segs.len() - 1].to_vec();
    Some((parent, idx))
}

/// Return the actions slice that `parent_path` points to.
///
/// Walking the tree is necessary because `parent_path` may descend through
/// Conditional branches. Returns `None` for an empty (outer-pipeline) path
/// without walking -- the caller treats `None` as "use root directly."
fn slice_at_parent<'a>(root: &'a [Action], parent_path: &[StageIdSegment]) -> Option<&'a [Action]> {
    // Empty parent path means the stage lives directly in root_actions.
    if parent_path.is_empty() {
        return None; // Signal "use root" to the caller.
    }

    // Walk: the parent_path is [Index(i), Branch, Index(j), Branch, ...],
    // always ending at a branch segment (IfTrue / IfFalse) because
    // the last Index segment was already stripped by split_stage_path.
    let mut cursor: &[Action] = root;
    let mut staged: Option<&Action> = None;
    let mut iter = parent_path.iter().peekable();
    while let Some(seg) = iter.next() {
        match seg {
            StageIdSegment::Index(i) => {
                let action = cursor.get(*i)?;
                // Shouldn't happen (path should end on a branch) but
                // handle gracefully: peek is None means we've exhausted
                // the path on an Index segment, which is malformed for
                // a parent_path.
                iter.peek()?;
                staged = Some(action);
            }
            StageIdSegment::IfTrue => match staged? {
                Action::Conditional { if_true, .. } => {
                    cursor = if_true.as_slice();
                    staged = None;
                }
                _ => return None,
            },
            StageIdSegment::IfFalse => match staged? {
                Action::Conditional { if_false, .. } => {
                    cursor = if_false.as_deref()?;
                    staged = None;
                }
                _ => return None,
            },
        }
    }
    Some(cursor)
}

/// Build a `StageId` from `parent_path + Index(idx)`.
fn make_stage_id(parent_path: &[StageIdSegment], idx: usize) -> StageId {
    let mut segs = parent_path.to_vec();
    segs.push(StageIdSegment::Index(idx));
    StageId(segs)
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Right-click stage actions menu.
///
/// Reads `EditorState::stage_menu`; renders nothing when `None`. When
/// `Some`, positions a small context menu at the stored cursor coordinates
/// with three items: Move up, Move down, Delete.
#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub(crate) fn StageActionsMenu(
    /// `(mode, InputAddress)` key for the mapping being edited.
    /// Named `mapping_key` to avoid collision with Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    /// Mapping's outermost actions vec (root-relative, same invariant as Pipeline).
    root_actions: Vec<Action>,
) -> Element {
    let editor = use_context::<EditorState>();
    let ctx = use_context::<AppContext>();

    let menu_state: Option<StageMenuState> = editor.stage_menu.read().clone();
    let Some(menu) = menu_state else {
        return rsx! {};
    };

    // ---------------------------------------------------------------------------
    // Derive boundary flags
    // ---------------------------------------------------------------------------

    let stage_id = menu.stage.clone();
    let Some((parent_path, current_idx)) = split_stage_path(&stage_id) else {
        // Malformed path: close and bail.
        let mut stage_menu = editor.stage_menu;
        stage_menu.set(None);
        return rsx! {};
    };

    let parent_len = match slice_at_parent(&root_actions, &parent_path) {
        None => root_actions.len(), // stage lives at root level
        Some(slice) => slice.len(),
    };

    let move_up_disabled = current_idx == 0;
    let move_down_disabled = current_idx + 1 >= parent_len;

    // ---------------------------------------------------------------------------
    // Action label (for undo entries)
    // ---------------------------------------------------------------------------

    let stage_action = {
        let id = stage_id.clone();
        // at_path lives in pipeline::mod, but we can walk manually since we
        // have parent_path + current_idx. Simplest: call at_path via the re-export.
        crate::frame::mapping_editor::pipeline::at_path(&root_actions, &id)
    };
    let stage_name: &'static str = stage_action.map_or("stage", stage_title_for);

    // ---------------------------------------------------------------------------
    // Move-up handler
    // ---------------------------------------------------------------------------

    let on_move_up = {
        let key = mapping_key.clone();
        let stage_id = stage_id.clone();
        let parent_path = parent_path.clone();
        let root = root_actions.clone();
        let cmd_tx = ctx.commands.clone();
        let cfg_sig = ctx.config;
        let mut undo_log = editor.undo_log;
        let mut stage_menu = editor.stage_menu;
        let mut expanded = editor.expanded_stages;
        let mut malformed = editor.malformed_hints;

        move |_: MouseEvent| {
            if move_up_disabled {
                return;
            }

            // Build sibling id (current_idx - 1).
            let prev_id = make_stage_id(&parent_path, current_idx - 1);

            // Swap: step1 = set slot (current_idx - 1) <- current action
            //       step2 = set slot current_idx       <- prev action
            let Some(current_action) =
                crate::frame::mapping_editor::pipeline::at_path(&root, &stage_id).cloned()
            else {
                return;
            };
            let Some(prev_action) =
                crate::frame::mapping_editor::pipeline::at_path(&root, &prev_id).cloned()
            else {
                return;
            };

            let Some(step1) = replace_at_path(&root, &prev_id, current_action) else {
                return;
            };
            let Some(new_actions) = replace_at_path(&step1, &stage_id, prev_action) else {
                return;
            };

            // Amendment 3: name from live snapshot.
            let cfg = cfg_sig.read();
            let current_name = cfg.mapping_names.get(&key.1).cloned();
            drop(cfg);

            let before = Mapping {
                input: key.1.clone(),
                mode: key.0.clone(),
                name: current_name.clone(),
                actions: root.clone(),
            };

            // Dispatch first; undo only on success.
            if cmd_tx
                .send(EngineCommand::SetMapping {
                    input: key.1.clone(),
                    mode: key.0.clone(),
                    name: current_name,
                    actions: new_actions,
                })
                .is_err()
            {
                tracing::warn!(
                    target: "f9::mapping_editor",
                    action = "stage_move_up_drop_offline",
                    "stage move-up dropped: engine channel disconnected"
                );
                stage_menu.set(None);
                return;
            }

            let label = format_undo_label(
                UndoKind::StageReorder,
                LabelArgs {
                    stage_name: Some(stage_name),
                    from_to: Some((current_idx, current_idx - 1)),
                    ..LabelArgs::default()
                },
            );
            undo_log
                .write()
                .push_edit(key.clone(), before, UndoKind::StageReorder, label);

            // Clear positional caches (Task 11 structural-mutation invariant).
            expanded.write().clear();
            malformed.write().clear();

            stage_menu.set(None);
        }
    };

    // ---------------------------------------------------------------------------
    // Move-down handler
    // ---------------------------------------------------------------------------

    let on_move_down = {
        let key = mapping_key.clone();
        let stage_id = stage_id.clone();
        let parent_path = parent_path.clone();
        let root = root_actions.clone();
        let cmd_tx = ctx.commands.clone();
        let cfg_sig = ctx.config;
        let mut undo_log = editor.undo_log;
        let mut stage_menu = editor.stage_menu;
        let mut expanded = editor.expanded_stages;
        let mut malformed = editor.malformed_hints;

        move |_: MouseEvent| {
            if move_down_disabled {
                return;
            }

            // Build sibling id (current_idx + 1).
            let next_id = make_stage_id(&parent_path, current_idx + 1);

            let Some(current_action) =
                crate::frame::mapping_editor::pipeline::at_path(&root, &stage_id).cloned()
            else {
                return;
            };
            let Some(next_action) =
                crate::frame::mapping_editor::pipeline::at_path(&root, &next_id).cloned()
            else {
                return;
            };

            // Swap: step1 = set slot (current_idx + 1) <- current action
            //       step2 = set slot current_idx       <- next action
            let Some(step1) = replace_at_path(&root, &next_id, current_action) else {
                return;
            };
            let Some(new_actions) = replace_at_path(&step1, &stage_id, next_action) else {
                return;
            };

            // Amendment 3: name from live snapshot.
            let cfg = cfg_sig.read();
            let current_name = cfg.mapping_names.get(&key.1).cloned();
            drop(cfg);

            let before = Mapping {
                input: key.1.clone(),
                mode: key.0.clone(),
                name: current_name.clone(),
                actions: root.clone(),
            };

            if cmd_tx
                .send(EngineCommand::SetMapping {
                    input: key.1.clone(),
                    mode: key.0.clone(),
                    name: current_name,
                    actions: new_actions,
                })
                .is_err()
            {
                tracing::warn!(
                    target: "f9::mapping_editor",
                    action = "stage_move_down_drop_offline",
                    "stage move-down dropped: engine channel disconnected"
                );
                stage_menu.set(None);
                return;
            }

            let label = format_undo_label(
                UndoKind::StageReorder,
                LabelArgs {
                    stage_name: Some(stage_name),
                    from_to: Some((current_idx, current_idx + 1)),
                    ..LabelArgs::default()
                },
            );
            undo_log
                .write()
                .push_edit(key.clone(), before, UndoKind::StageReorder, label);

            // Clear positional caches (Task 11 structural-mutation invariant).
            expanded.write().clear();
            malformed.write().clear();

            stage_menu.set(None);
        }
    };

    // ---------------------------------------------------------------------------
    // Delete handler
    // ---------------------------------------------------------------------------

    let on_delete = {
        let key = mapping_key.clone();
        let stage_id = stage_id.clone();
        let root = root_actions.clone();
        let cmd_tx = ctx.commands.clone();
        let cfg_sig = ctx.config;
        let mut undo_log = editor.undo_log;
        let mut stage_menu = editor.stage_menu;
        let mut expanded = editor.expanded_stages;
        let mut malformed = editor.malformed_hints;

        move |_: MouseEvent| {
            let Some(new_actions) = remove_at_path(&root, &stage_id) else {
                // Invalid path: skip the edit + skip phantom undo entry.
                stage_menu.set(None);
                return;
            };

            // Amendment 3: name from live snapshot.
            let cfg = cfg_sig.read();
            let current_name = cfg.mapping_names.get(&key.1).cloned();
            drop(cfg);

            let before = Mapping {
                input: key.1.clone(),
                mode: key.0.clone(),
                name: current_name.clone(),
                actions: root.clone(),
            };

            if cmd_tx
                .send(EngineCommand::SetMapping {
                    input: key.1.clone(),
                    mode: key.0.clone(),
                    name: current_name,
                    actions: new_actions,
                })
                .is_err()
            {
                tracing::warn!(
                    target: "f9::mapping_editor",
                    action = "stage_delete_drop_offline",
                    "stage delete dropped: engine channel disconnected"
                );
                stage_menu.set(None);
                return;
            }

            let label = format_undo_label(
                UndoKind::StageRemove,
                LabelArgs {
                    stage_name: Some(stage_name),
                    index: Some(current_idx),
                    ..LabelArgs::default()
                },
            );
            undo_log
                .write()
                .push_edit(key.clone(), before, UndoKind::StageRemove, label);

            // Clear positional caches (Task 11 structural-mutation invariant).
            expanded.write().clear();
            malformed.write().clear();

            stage_menu.set(None);
        }
    };

    // ---------------------------------------------------------------------------
    // Close-on-backdrop handler
    // ---------------------------------------------------------------------------

    let on_backdrop = {
        let mut stage_menu = editor.stage_menu;
        move |_: MouseEvent| {
            stage_menu.set(None);
        }
    };

    // ---------------------------------------------------------------------------
    // Render
    // ---------------------------------------------------------------------------

    // `position: fixed` anchors the menu at cursor coordinates regardless of
    // scroll position. z-index 100 lifts it above all editor content (pipeline
    // cards are unstyled stack order; add-palette uses z-index 5).
    let anchor_style = format!(
        "position: fixed; left: {}px; top: {}px; z-index: 100;",
        menu.x, menu.y
    );

    rsx! {
        // Full-viewport transparent backdrop dismisses the menu on outside click.
        div {
            class: "if-stage-menu__backdrop",
            "aria-hidden": "true",
            onclick: on_backdrop,
        }
        div {
            class: "if-stage-menu-anchor",
            style: "{anchor_style}",
            div {
                class: "if-stage-menu",
                role: "menu",
                button {
                    r#type: "button",
                    role: "menuitem",
                    class: "if-stage-menu__item",
                    "aria-disabled": "{move_up_disabled}",
                    disabled: move_up_disabled,
                    onclick: on_move_up,
                    "Move up"
                }
                button {
                    r#type: "button",
                    role: "menuitem",
                    class: "if-stage-menu__item",
                    "aria-disabled": "{move_down_disabled}",
                    disabled: move_down_disabled,
                    onclick: on_move_down,
                    "Move down"
                }
                button {
                    r#type: "button",
                    role: "menuitem",
                    class: "if-stage-menu__item if-stage-menu__item--danger",
                    onclick: on_delete,
                    "Delete"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests for the private helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id(segs: Vec<StageIdSegment>) -> StageId {
        StageId(segs)
    }

    #[test]
    fn split_outer_index_zero() {
        let id = make_id(vec![StageIdSegment::Index(0)]);
        let (parent, idx) = split_stage_path(&id).unwrap();
        assert!(parent.is_empty());
        assert_eq!(idx, 0);
    }

    #[test]
    fn split_outer_index_two() {
        let id = make_id(vec![StageIdSegment::Index(2)]);
        let (parent, idx) = split_stage_path(&id).unwrap();
        assert!(parent.is_empty());
        assert_eq!(idx, 2);
    }

    #[test]
    fn split_nested_into_if_true() {
        let id = make_id(vec![
            StageIdSegment::Index(1),
            StageIdSegment::IfTrue,
            StageIdSegment::Index(3),
        ]);
        let (parent, idx) = split_stage_path(&id).unwrap();
        assert_eq!(
            parent,
            vec![StageIdSegment::Index(1), StageIdSegment::IfTrue]
        );
        assert_eq!(idx, 3);
    }

    #[test]
    fn split_empty_path_returns_none() {
        let id = make_id(vec![]);
        assert!(split_stage_path(&id).is_none());
    }

    #[test]
    fn make_stage_id_round_trips() {
        let parent = vec![StageIdSegment::Index(0), StageIdSegment::IfTrue];
        let id = make_stage_id(&parent, 2);
        let (back_parent, idx) = split_stage_path(&id).unwrap();
        assert_eq!(back_parent, parent);
        assert_eq!(idx, 2);
    }
}
