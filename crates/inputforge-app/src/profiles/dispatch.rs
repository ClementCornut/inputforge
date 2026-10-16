use super::{Scope, Session, policy};
use inputforge_core::{engine::EngineCommand, state::AppState};
use inputforge_gui_dx::profiles::{Intent, Mode, Source, View};
use std::sync::mpsc;

pub(super) fn current(session: &Session, state: &AppState) -> Result<(), String> {
    if session.scope != Scope::from_state(state) {
        return Err("Profile changed. Cancel and reopen this action.".into());
    }
    Ok(())
}
pub(super) fn send(
    commands: &mpsc::Sender<EngineCommand>,
    command: EngineCommand,
) -> Result<(), String> {
    commands
        .send(command)
        .map_err(|_closed| "Could not send profile action. Try again.".into())
}
pub(super) fn close(session: &mut Session, view: &mut View) {
    session.picker += 1;
    view.workflow += 1;
    view.mode = Mode::Library;
    view.rename = None;
    view.delete = None;
}
pub(super) fn prepare(
    session: &mut Session,
    view: &mut View,
    state: &AppState,
    intent: Intent,
) -> Result<Option<EngineCommand>, String> {
    let command = match intent {
        Intent::Snapshot(intent) => return super::snapshots::prepare(view, state, intent),
        Intent::Back => {
            close(session, view);
            session.scope = Scope::from_state(state);
            None
        }
        Intent::CancelRename => {
            view.rename = None;
            view.workflow += 1;
            None
        }
        Intent::CancelDelete => {
            view.delete = None;
            view.workflow += 1;
            None
        }
        Intent::Name(name) => {
            view.name = name;
            None
        }
        Intent::Filter(filter) => {
            view.filter = filter;
            None
        }
        Intent::Source(source) => {
            view.source = source;
            session.picker += 1;
            None
        }
        Intent::LibrarySource { key, choices } => {
            view.source = Source::Library(choices.iter().find(|t| t.key == key).cloned());
            None
        }
        Intent::New => {
            close(session, view);
            session.scope = Scope::from_state(state);
            view.mode = Mode::New;
            view.name = "New Profile".into();
            view.source = Source::Blank;
            None
        }
        Intent::BeginRename(t) => {
            policy::library(&t, state)?;
            close(session, view);
            session.scope = Scope::from_state(state);
            view.name.clone_from(&t.name);
            view.rename = Some(t);
            None
        }
        Intent::BeginDelete(t) => {
            policy::library(&t, state)?;
            close(session, view);
            session.scope = Scope::from_state(state);
            view.delete = Some(t);
            None
        }
        Intent::Create => super::forms::create(session, view, state)?,
        Intent::Rename => super::forms::rename(session, view, state)?,
        Intent::Delete => super::forms::delete(session, view, state)?,
        Intent::LoadOnce | Intent::Add => super::forms::open(&intent, session, view, state)?,
        Intent::Open(t) => {
            policy::current(&t, state)?;
            Some(EngineCommand::LoadProfile(t.path))
        }
        Intent::Reveal(t) => {
            policy::current(&t, state)?;
            Some(EngineCommand::RevealProfile { path: t.path })
        }
        Intent::Duplicate(t) => {
            policy::library(&t, state)?;
            let name = policy::validate(&format!("{} Copy", t.name), None, state)?;
            Some(EngineCommand::DuplicateProfile {
                source_path: t.path,
                name,
            })
        }
        Intent::AddRow(t) => {
            policy::current(&t, state)?;
            if state.active_profile_origin != Some(inputforge_core::state::ProfileOrigin::External)
                || state.profile_path.as_ref().is_none_or(|p| *p != t.path)
            {
                return Err("This action requires the active external profile.".into());
            }
            let name = policy::validate(&t.name, None, state)?;
            Some(EngineCommand::AddExternalProfileToLibrary { path: t.path, name })
        }
        Intent::Pick => return Err("Picker request was not started.".into()),
    };
    Ok(command)
}
