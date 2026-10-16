//! Profile form preparation reuses core name and library policy.
use super::{Session, policy};
use inputforge_core::{engine::EngineCommand, state::AppState};
use inputforge_gui_dx::profiles::{Intent, Mode, Source, View};
pub(super) fn create(
    session: &mut Session,
    view: &mut View,
    state: &AppState,
) -> Result<Option<EngineCommand>, String> {
    if view.mode != Mode::New {
        return Err("Reopen the new profile form.".into());
    }
    super::dispatch::current(session, state)?;
    let name = policy::validate(&view.name, None, state)?;
    let command = match &view.source {
        Source::Blank => EngineCommand::CreateProfile { name },
        Source::Active => EngineCommand::DuplicateProfile {
            source_path: state
                .profile_path
                .clone()
                .ok_or("Pick a source profile first.")?,
            name,
        },
        Source::Library(Some(t)) => {
            policy::library(t, state)?;
            EngineCommand::DuplicateProfile {
                source_path: t.path.clone(),
                name,
            }
        }
        Source::Library(None) | Source::File => {
            return Err("Pick a profile file first.".into());
        }
    };
    super::dispatch::close(session, view);
    Ok(Some(command))
}
pub(super) fn rename(
    session: &mut Session,
    view: &mut View,
    state: &AppState,
) -> Result<Option<EngineCommand>, String> {
    super::dispatch::current(session, state)?;
    let t = view.rename.as_ref().ok_or("Reopen the rename field.")?;
    policy::library(t, state)?;
    let name = policy::validate(&view.name, Some(t), state)?;
    let cmd = (name != t.name).then(|| EngineCommand::RenameProfile {
        old_name: t.name.clone(),
        new_name: name,
    });
    view.rename = None;
    view.workflow += 1;
    Ok(cmd)
}
pub(super) fn delete(
    session: &mut Session,
    view: &mut View,
    state: &AppState,
) -> Result<Option<EngineCommand>, String> {
    super::dispatch::current(session, state)?;
    let t = view
        .delete
        .as_ref()
        .ok_or("Reopen the delete confirmation.")?;
    policy::library(t, state)?;
    let cmd = EngineCommand::DeleteProfile {
        name: t.name.clone(),
    };
    view.delete = None;
    view.workflow += 1;
    Ok(Some(cmd))
}
pub(super) fn open(
    intent: &Intent,
    session: &mut Session,
    view: &mut View,
    state: &AppState,
) -> Result<Option<EngineCommand>, String> {
    super::dispatch::current(session, state)?;
    let Mode::Open { path } = &view.mode else {
        return Err("Pick a profile file first.".into());
    };
    if path.as_os_str().is_empty() {
        return Err("Pick a profile file first.".into());
    }
    let cmd = if *intent == Intent::LoadOnce {
        EngineCommand::LoadExternalProfileOnce(path.clone())
    } else {
        EngineCommand::AddExternalProfileToLibrary {
            path: path.clone(),
            name: policy::validate(&view.name, None, state)?,
        }
    };
    super::dispatch::close(session, view);
    Ok(Some(cmd))
}
