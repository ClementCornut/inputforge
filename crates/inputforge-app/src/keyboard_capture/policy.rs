//! Current mapping and action-path validation for keyboard destinations.
use dioxus::prelude::ReadableExt;
use inputforge_core::{
    action::{Action, branch_actions},
    pipeline::ActionPathSegment,
    state::AppState,
};
use inputforge_gui_dx::keyboard_capture::{Destination, Publication, Target};
pub(super) fn navigation(p: Publication, target: &Target) -> bool {
    let t = &target.mapping;
    p.mode.try_peek().is_ok_and(|mode| *mode == t.mode)
        && p.selection.try_peek().is_ok_and(|selection| {
            selection
                .as_ref()
                .is_some_and(|(mode, input)| mode == &t.mode && t.mapping.as_ref() == Some(input))
        })
}
pub(super) fn valid(state: &AppState, target: &Target) -> bool {
    let t = &target.mapping;
    let Some(profile) = state.active_profile.as_ref() else {
        return false;
    };
    if t.profile.as_ref() != Some(profile.id()) || !profile.modes().contains(&t.mode) {
        return false;
    }
    let Some(mapping) = t
        .mapping
        .as_ref()
        .and_then(|input| profile.find_mapping(input, &t.mode))
    else {
        return false;
    };
    if t.actions.as_ref() != Some(&mapping.actions) || t.name != mapping.name {
        return false;
    }
    match &target.destination {
        Destination::Edit(path) => {
            position(&mapping.actions, path).is_some_and(|(branch, index)| {
                matches!(branch.get(index), Some(Action::MapToKeyboard { .. }))
            })
        }
        Destination::Insert(path) => {
            position(&mapping.actions, path).is_some_and(|(branch, index)| index <= branch.len())
        }
    }
}
fn position<'a>(
    mut actions: &'a [Action],
    path: &[ActionPathSegment],
) -> Option<(&'a [Action], usize)> {
    let (ActionPathSegment::Index(index), parents) = path.split_last()? else {
        return None;
    };
    for pair in parents.chunks(2) {
        let [
            ActionPathSegment::Index(parent),
            ActionPathSegment::Branch(branch),
        ] = pair
        else {
            return None;
        };
        actions = branch_actions(actions.get(*parent)?, *branch)?;
    }
    Some((actions, *index))
}
