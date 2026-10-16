//! Coherent mode availability and shared core validation.
use inputforge_core::{engine::validate_mode_deletion, state::AppState};
use inputforge_gui_dx::mode_management::{Row, View};
pub(crate) fn view(state: &AppState) -> View {
    let Some(profile) = state.active_profile.as_ref() else {
        return View::default();
    };
    profile_view(state, profile)
}
pub(super) fn profile_view(state: &AppState, profile: &inputforge_core::profile::Profile) -> View {
    let online = !state.session.offline;
    View {
        revision: state.mode_revision,
        can_add: online,
        rows: profile
            .modes()
            .as_slice()
            .iter()
            .map(|name| Row {
                name: name.clone(),
                running: state.current_mode == *name,
                can_activate: online && state.current_mode != *name,
                can_rename: online,
                can_delete: online && validate_mode_deletion(profile, name).is_ok(),
                can_default: online && profile.settings().startup_mode() != name,
                mappings: profile
                    .mappings()
                    .iter()
                    .filter(|m| m.mode == *name)
                    .count(),
            })
            .collect(),
    }
}
