//! Profile name, namespace and row selection policy.
use inputforge_core::{
    profile::manager::{sanitize_filename, validate_profile_name},
    state::AppState,
};
use inputforge_gui_dx::{
    profiles::{Row, Source, Target, View},
    snapshots::ProfileRowOrigin,
};
use std::fmt::Write as _;

pub(super) fn target(path: &std::path::Path, state: &AppState) -> Result<Target, String> {
    let active = state.profile_path.as_ref().is_some_and(|p| p == path);
    let row = state.profile_library_rows.iter().find(|r| r.path == path);
    let name = row
        .map(|r| r.name.clone())
        .or_else(|| {
            active
                .then(|| state.active_profile.as_ref().map(|p| p.name().to_owned()))
                .flatten()
        })
        .ok_or("Profile is no longer available.")?;
    let profile = state
        .active_profile
        .as_ref()
        .filter(|_| active)
        .map(|p| p.id().clone());
    let mut key = String::new();
    for byte in path.as_os_str().as_encoded_bytes() {
        write!(&mut key, "{byte:02x}").expect("formatting a path token into String cannot fail");
    }
    Ok(Target {
        path: path.to_path_buf(),
        key,
        name,
        profile,
        library_profile: row.and_then(|r| r.profile_id.clone()),
    })
}
pub(super) fn current(captured: &Target, state: &AppState) -> Result<(), String> {
    if target(&captured.path, state)? != *captured {
        return Err("Profile changed. Cancel and reopen this action.".into());
    }
    Ok(())
}
pub(super) fn library(captured: &Target, state: &AppState) -> Result<(), String> {
    current(captured, state)?;
    if !crate::bridge::meta::native_profile_rows(state)
        .iter()
        .any(|(path, r)| *path == captured.path && r.origin == ProfileRowOrigin::Library)
    {
        return Err("This action requires a library profile.".into());
    }
    Ok(())
}
pub(super) fn validate(
    name: &str,
    old: Option<&Target>,
    state: &AppState,
) -> Result<String, String> {
    let name = name.trim();
    validate_profile_name(name).map_err(|e| e.to_string())?;
    let filename = sanitize_filename(name);
    if filename.is_empty() {
        return Err("Name is empty after filename sanitization.".into());
    }
    if state
        .profile_library_rows
        .iter()
        .filter(|r| old.is_none_or(|o| r.path != o.path))
        .any(|r| {
            r.name.eq_ignore_ascii_case(name)
                || sanitize_filename(&r.name).eq_ignore_ascii_case(&filename)
        })
    {
        return Err("A profile with this name already exists.".into());
    }
    Ok(name.into())
}
pub(super) fn project(state: &AppState, view: &mut View) {
    super::snapshots::project(state, view);
    let rows = crate::bridge::meta::native_profile_rows(state);
    view.library = rows
        .iter()
        .filter(|(_, r)| r.origin == ProfileRowOrigin::Library)
        .filter_map(|(path, _)| target(path, state).ok())
        .collect();
    let needle = view.filter.trim().to_lowercase();
    let previous = std::mem::take(&mut view.rows);
    view.rows = rows
        .into_iter()
        .filter_map(|(path, mut row)| {
            row.is_active = state.profile_path.as_ref() == Some(&path);
            row.can_snapshot_now = row.is_active && state.active_profile.is_some();
            if !row.is_active && !needle.is_empty() && !row.name.to_lowercase().contains(&needle) {
                return None;
            }
            let target = target(&path, state).ok()?;
            if !row.can_open
                && row.last_edited_label.is_none()
                && let Some(old) = previous.iter().find(|old| old.target == target)
            {
                row.last_edited_label.clone_from(&old.row.last_edited_label);
            }
            Some(Row { row, target })
        })
        .collect();
    view.rows
        .sort_by_key(|r| (!r.row.is_active, r.row.name.to_lowercase()));
    view.filtered_empty =
        !view.filter.trim().is_empty() && !view.rows.iter().any(|r| !r.row.is_active);
    view.name_error = match view.mode {
        inputforge_gui_dx::profiles::Mode::New | inputforge_gui_dx::profiles::Mode::Open { .. } => {
            validate(&view.name, None, state).err()
        }
        inputforge_gui_dx::profiles::Mode::Library => view
            .rename
            .as_ref()
            .and_then(|t| validate(&view.name, Some(t), state).err()),
    };
    let valid = validate(&view.name, None, state).is_ok();
    view.can_add = valid;
    view.can_create = valid
        && match &view.source {
            Source::Blank => true,
            Source::Active => state.profile_path.is_some(),
            Source::Library(Some(t)) => library(t, state).is_ok(),
            Source::Library(None) | Source::File => false,
        };
    view.deleting_active = view
        .delete
        .as_ref()
        .is_some_and(|t| state.profile_path.as_ref() == Some(&t.path));
}
