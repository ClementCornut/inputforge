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

use inputforge_core::action::Action;

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

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::action::Condition;
    use inputforge_core::types::{DeviceId, InputAddress, InputId, MergeOp};

    fn synth_addr() -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

    #[test]
    fn at_path_outer_index() {
        let actions = vec![Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(0)]);
        assert!(matches!(at_path(&actions, &path), Some(Action::Invert)));
    }

    #[test]
    fn at_path_into_if_true_branch() {
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: synth_addr(),
            },
            if_true: vec![Action::Invert],
            if_false: None,
        }];
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfTrue,
            StageIdSegment::Index(0),
        ]);
        assert!(matches!(at_path(&actions, &path), Some(Action::Invert)));
    }

    #[test]
    fn at_path_into_missing_if_false_returns_none() {
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: synth_addr(),
            },
            if_true: vec![],
            if_false: None,
        }];
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfFalse,
            StageIdSegment::Index(0),
        ]);
        assert!(at_path(&actions, &path).is_none());
    }

    #[test]
    fn replace_at_path_outer_swaps_action() {
        let actions = vec![Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(0)]);
        let new = replace_at_path(
            &actions,
            &path,
            Action::MergeAxis {
                second_input: synth_addr(),
                operation: MergeOp::Average,
            },
        )
        .expect("valid path must succeed");
        assert!(matches!(new[0], Action::MergeAxis { .. }));
    }

    #[test]
    fn replace_at_path_inside_if_true_swaps_action() {
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: synth_addr(),
            },
            if_true: vec![Action::Invert],
            if_false: None,
        }];
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfTrue,
            StageIdSegment::Index(0),
        ]);
        let new = replace_at_path(
            &actions,
            &path,
            Action::MergeAxis {
                second_input: synth_addr(),
                operation: MergeOp::Average,
            },
        )
        .expect("valid path must succeed");
        match &new[0] {
            Action::Conditional { if_true, .. } => {
                assert!(matches!(if_true[0], Action::MergeAxis { .. }));
            }
            _ => panic!("outer wrapper should remain Conditional"),
        }
    }

    #[test]
    fn replace_at_path_invalid_path_returns_none() {
        // Out-of-range index: must return None, not panic, in BOTH debug
        // and release. Callers depend on this to skip the edit + skip
        // push_edit (no phantom undo entries).
        let actions = vec![Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(99)]);
        assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

        // Empty path.
        let path = StageId(vec![]);
        assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

        // Path starts with a branch segment.
        let path = StageId(vec![StageIdSegment::IfTrue]);
        assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

        // Branch segment after a non-Conditional action.
        let path = StageId(vec![StageIdSegment::Index(0), StageIdSegment::IfTrue]);
        assert!(replace_at_path(&actions, &path, Action::Invert).is_none());
    }

    #[test]
    fn insert_at_path_outer_appends() {
        let actions = vec![Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(1)]);
        let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
        assert_eq!(new.len(), 2);
    }

    #[test]
    fn insert_at_path_outer_inserts_at_index() {
        let actions = vec![Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(0)]);
        let new = insert_at_path(
            &actions,
            &path,
            Action::MergeAxis {
                second_input: synth_addr(),
                operation: MergeOp::Average,
            },
        )
        .expect("valid path");
        assert_eq!(new.len(), 2);
        assert!(matches!(new[0], Action::MergeAxis { .. }));
        assert!(matches!(new[1], Action::Invert));
    }

    #[test]
    fn insert_at_path_into_if_false_creates_branch() {
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: synth_addr(),
            },
            if_true: vec![],
            if_false: None,
        }];
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfFalse,
            StageIdSegment::Index(0),
        ]);
        let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
        match &new[0] {
            Action::Conditional { if_false, .. } => {
                assert_eq!(if_false.as_ref().map(Vec::len), Some(1));
            }
            _ => panic!("expected Conditional"),
        }
    }

    #[test]
    fn remove_at_path_outer_drops_action() {
        let actions = vec![Action::Invert, Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(0)]);
        let new = remove_at_path(&actions, &path).expect("valid path");
        assert_eq!(new.len(), 1);
    }

    #[test]
    fn remove_at_path_last_in_if_false_collapses_to_none() {
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: synth_addr(),
            },
            if_true: vec![],
            if_false: Some(vec![Action::Invert]),
        }];
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfFalse,
            StageIdSegment::Index(0),
        ]);
        let new = remove_at_path(&actions, &path).expect("valid path");
        match &new[0] {
            Action::Conditional { if_false, .. } => {
                assert!(
                    if_false.is_none(),
                    "empty if_false branch must collapse to None"
                );
            }
            _ => panic!("expected Conditional"),
        }
    }

    #[test]
    fn insert_remove_invalid_paths_return_none() {
        // Same contract as replace_at_path: callers depend on None
        // (NOT panic in release) so they can skip the edit + skip push_edit.
        let actions = vec![Action::Invert];

        // Empty path.
        assert!(insert_at_path(&actions, &StageId(vec![]), Action::Invert).is_none());
        assert!(remove_at_path(&actions, &StageId(vec![])).is_none());

        // Path starts with branch segment.
        assert!(
            insert_at_path(
                &actions,
                &StageId(vec![StageIdSegment::IfTrue]),
                Action::Invert
            )
            .is_none()
        );
        assert!(remove_at_path(&actions, &StageId(vec![StageIdSegment::IfTrue])).is_none());

        // Out-of-range index for remove_at_path.
        assert!(remove_at_path(&actions, &StageId(vec![StageIdSegment::Index(99)])).is_none());

        // Branch segment after a non-Conditional action.
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfTrue,
            StageIdSegment::Index(0),
        ]);
        assert!(insert_at_path(&actions, &path, Action::Invert).is_none());
        assert!(remove_at_path(&actions, &path).is_none());
    }
}
