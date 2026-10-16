//! App-owned instrument drafts, input policy and mounted listener ownership.
mod curve;
mod deadzone;
mod keyboard;
mod lifecycle;
#[cfg(test)]
mod pointer_fixture;
mod response_curve;
mod session;
mod zone;
use super::{Controller, history, paths};
use dioxus::prelude::*;
use inputforge_core::action::{Action, Mapping};
use inputforge_gui_dx::{
    instruments::{Bindings, Event, Open, View},
    mapping_edits::{Edit, Request},
};
pub(super) use session::Session;
use std::time::Instant;

impl Controller {
    pub(super) fn instrument_bindings(&self) -> Bindings {
        let open = self.clone();
        let events = self.clone();
        let close = self.clone();
        let mount = self.clone();
        Bindings {
            open: Callback::new(move |request| open.open_instrument(request)),
            event: Callback::new(move |(id, event)| event_instrument(&events, id, &event)),
            close: Callback::new(move |id| close.close_instrument(id)),
            mount: Callback::new(move |request| mount.mount_instrument(&request)),
        }
    }
    fn open_instrument(&self, open: Open) -> View {
        self.reconcile();
        let id = uuid::Uuid::new_v4().as_u128();
        let revision = self.inner.borrow().revision;
        let mut session = Session::new(open, revision);
        if let Some(current) = self.instrument_current(&session) {
            let captured = session.open.target.as_ref().map(|t| &t.before);
            let state = self.state.read();
            let inner = self.inner.borrow();
            let key = paths::key(&current);
            let accepted = captured.is_some_and(|before| {
                history::saved(&state, &key).as_ref() == Some(before)
                    || &current == before
                    || inner.history.accepts_stale(&key, before)
            });
            if accepted && !inner.history.structural_pending(&key) {
                session.adopt(current);
            } else {
                session.available = false;
            }
        } else {
            session.available = false;
        }
        let view = session.view(id);
        self.inner.borrow_mut().instruments.insert(id, session);
        view
    }
    fn instrument_current(&self, session: &Session) -> Option<Mapping> {
        let state = self.state.read();
        if self.mode_pending(&state) || state.session.offline {
            return None;
        }
        let inner = self.inner.borrow();
        if inner.disposed {
            return None;
        }
        let target = session.open.target.as_ref()?;
        if state.active_profile.as_ref()?.id() != target.scope.profile.as_ref()? {
            return None;
        }
        let key = paths::key(&target.before);
        if let Some(p) = inner.publication
            && (*p.mode.peek() != key.0 || p.selection.peek().as_ref() != Some(&key))
        {
            return None;
        }
        inner.history.current(&state, &key)
    }
}
fn event_instrument(controller: &Controller, id: u128, event: &Event) -> Option<View> {
    controller.reconcile();
    let mut session = controller.inner.borrow_mut().instruments.remove(&id)?;
    session.prevent_default = false;
    let current = controller.instrument_current(&session);
    let revision = controller.inner.borrow().revision;
    let changed = session.expected.as_ref() != current.as_ref() || session.revision != revision;
    match current.filter(|_| session.available) {
        None => {
            session.cancel();
            session.available = false;
            session.error =
                Some("This instrument is no longer available. Reopen the current stage.".into());
        }
        Some(current) if changed => {
            session.adopt(current);
            session.revision = revision;
            if !matches!(event, Event::Refresh) {
                session.error =
                    Some("The mapping changed. Retry on the current instrument.".into());
            }
        }
        Some(_) => process(controller, &mut session, event),
    }
    let view = session.view(id);
    controller
        .inner
        .borrow_mut()
        .instruments
        .insert(id, session);
    Some(view)
}
fn process(controller: &Controller, session: &mut Session, event: &Event) {
    session.prevent_default = false;
    if matches!(event, Event::Refresh) {
        return;
    }
    if matches!(event, Event::Cancel) || matches!(event,Event::Pointer{kind,..} if kind=="cancel") {
        session.cancel();
        controller.end_group();
        return;
    }
    if matches!(event, Event::Blur) {
        controller.end_group();
        return;
    }
    let commit = match &session.value {
        Action::ResponseCurve { .. } => curve::event(session, event),
        Action::Deadzone { .. } => zone::event(session, event),
        _ => None,
    };
    if let Event::Key { key, .. } = event
        && (session.prevent_default || key == "Tab")
        && !keyboard::is_nudge(event, &session.value)
    {
        controller.end_group();
    }
    if let Some(commit) = commit {
        let Some(before) = session.expected.clone() else {
            return;
        };
        if paths::at(&before.actions, &session.open.path.0) == Some(&commit.action) {
            session.draft = false;
            session.error = None;
            return;
        }
        let result = controller.edit(
            Request {
                profile: session
                    .open
                    .target
                    .as_ref()
                    .and_then(|t| t.scope.profile.clone()),
                before,
                edit: Edit::Replace {
                    path: session.open.path.clone(),
                    action: commit.action,
                },
                label: commit.label,
                group: commit
                    .group
                    .map(|key| inputforge_gui_dx::mapping_edits::Group {
                        path: session.open.path.clone(),
                        key,
                    }),
                editor: true,
            },
            Instant::now(),
            true,
        );
        match result {
            Ok(_) => {
                session.expected = controller.instrument_current(session);
                session.revision = controller.inner.borrow().revision;
                session.draft = false;
                session.error = None;
            }
            Err(error) => {
                session.error = Some(error);
                session.draft = true;
            }
        }
    }
}
#[derive(Debug)]
struct Commit {
    action: Action,
    label: String,
    group: Option<String>,
}
