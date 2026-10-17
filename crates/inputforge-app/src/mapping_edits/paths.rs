//! Validated root-relative action tree edits.
use inputforge_core::action::{Action, ActionBranch, branch_actions, branch_actions_mut};
use inputforge_gui_dx::mapping_edits::{Edit, StageIdSegment};
fn branch<'a>(
    actions: &'a mut Vec<Action>,
    path: &[StageIdSegment],
) -> Option<(&'a mut Vec<Action>, usize)> {
    let (StageIdSegment::Index(index), parents) = path.split_last()? else {
        return None;
    };
    let mut cursor = actions;
    for pair in parents.chunks(2) {
        let [
            StageIdSegment::Index(parent),
            StageIdSegment::Branch(branch),
        ] = pair
        else {
            return None;
        };
        cursor = branch_actions_mut(cursor.get_mut(*parent)?, *branch)?;
    }
    Some((cursor, *index))
}
pub(super) fn apply(actions: &[Action], edit: &Edit) -> Option<Vec<Action>> {
    let mut actions = actions.to_vec();
    match edit {
        Edit::Replace { path, action } => {
            let (list, i) = branch(&mut actions, &path.0)?;
            *list.get_mut(i)? = action.clone();
        }
        Edit::Insert { path, action } => {
            let (list, i) = branch(&mut actions, &path.0)?;
            if i > list.len() {
                return None;
            }
            list.insert(i, action.clone());
        }
        Edit::Remove(path) => {
            let (list, i) = branch(&mut actions, &path.0)?;
            list.get(i)?;
            list.remove(i);
        }
        Edit::Move { from, to } => {
            // Dropping an ancestor into itself cannot form a valid positional path.
            if to.0.starts_with(&from.0) {
                return None;
            }
            let (list, i) = branch(&mut actions, &from.0)?;
            list.get(i)?;
            let action = list.remove(i);
            let (list, i) = branch(&mut actions, &to.0)?;
            if i > list.len() {
                return None;
            }
            list.insert(i, action);
        }
        Edit::MoveGap { from, parent, gap } => {
            let target = crate::ui::sortable::gap_destination(from, parent, *gap)?;
            let (list, index) = branch(&mut actions, &from.0)?;
            list.get(index)?;
            let action = list.remove(index);
            let (list, index) = branch(&mut actions, &target.0)?;
            if index > list.len() {
                return None;
            }
            list.insert(index, action);
        }
        Edit::Rename(_) | Edit::Rebind(_) => {}
        Edit::Field { .. } | Edit::Add { .. } | Edit::Step { .. } => return None,
    }
    Some(actions)
}
pub(super) fn key(
    mapping: &inputforge_core::action::Mapping,
) -> (String, inputforge_core::types::InputAddress) {
    (mapping.mode.clone(), mapping.input.clone())
}

pub(super) fn at<'a>(actions: &'a [Action], path: &[StageIdSegment]) -> Option<&'a Action> {
    let (StageIdSegment::Index(index), parents) = path.split_last()? else {
        return None;
    };
    let mut list = actions;
    for pair in parents.chunks(2) {
        let [
            StageIdSegment::Index(parent),
            StageIdSegment::Branch(branch),
        ] = pair
        else {
            return None;
        };
        list = branch_actions(list.get(*parent)?, *branch)?;
    }
    list.get(*index)
}
pub(super) fn same_branches(before: &Action, after: &Action) -> bool {
    [
        ActionBranch::ConditionalTrue,
        ActionBranch::ConditionalFalse,
        ActionBranch::TapSingle,
        ActionBranch::TapDouble,
        ActionBranch::PressShort,
        ActionBranch::PressLong,
    ]
    .into_iter()
    .all(|branch| branch_actions(before, branch) == branch_actions(after, branch))
}
pub(super) fn structural(actions: &[Action], edit: &Edit) -> bool {
    match edit {
        Edit::Replace { path, action } => {
            at(actions, &path.0).is_none_or(|old| !same_branches(old, action))
        }
        Edit::Rename(_) => false,
        _ => true,
    }
}
pub(super) fn preserves_children(before: &[Action], effective: &[Action], edit: &Edit) -> bool {
    if let Edit::Replace { path, .. } = edit {
        return at(before, &path.0)
            .zip(at(effective, &path.0))
            .is_some_and(|(old, new)| same_branches(old, new));
    }
    true
}
