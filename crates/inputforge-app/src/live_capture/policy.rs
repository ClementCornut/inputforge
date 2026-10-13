//! Revalidate capture targets and retain native layout identities.
use dioxus::prelude::*;
use inputforge_core::{
    profile::controllers::ControlLayout,
    state::AppState,
    types::{DeviceId, InputAddress},
};
use inputforge_gui_dx::capture::{Publication, Target};

pub(super) fn navigation(publication: Publication, target: &Target) -> bool {
    publication
        .mode
        .try_peek()
        .is_ok_and(|mode| *mode == target.mode)
        && target.mapping.as_ref().is_none_or(|input| {
            publication.selection.try_peek().is_ok_and(|selection| {
                selection
                    .as_ref()
                    .is_some_and(|(mode, selected)| mode == &target.mode && selected == input)
            })
        })
}

pub(super) fn valid(state: &AppState, target: &Target) -> bool {
    let Some(profile) = state.active_profile.as_ref() else {
        return false;
    };
    if state.session.offline
        || target.profile.as_ref() != Some(profile.id())
        || !profile.modes().contains(&target.mode)
    {
        return false;
    }
    match &target.mapping {
        None => true,
        Some(input) => profile
            .find_mapping(input, &target.mode)
            .is_some_and(|mapping| {
                target.actions.as_ref() == Some(&mapping.actions) && target.name == mapping.name
            }),
    }
}
pub(super) fn layouts(state: &AppState) -> Vec<(DeviceId, ControlLayout)> {
    state
        .session
        .bindings
        .iter()
        .map(|table| {
            (
                table.device.clone(),
                ControlLayout {
                    axes: table.axes.iter().map(|axis| axis.code).collect(),
                    buttons: table.buttons.clone(),
                    hats: table.hats.clone(),
                },
            )
        })
        .collect()
}
pub(super) fn present(state: &AppState, address: &InputAddress) -> bool {
    state
        .input_cache
        .clone_compact()
        .iter()
        .any(|entry| &entry.address == address)
}
