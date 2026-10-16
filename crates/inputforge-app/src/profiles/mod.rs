//! Shared profile workflow policy and native picker lifecycle.
mod dialog;
mod dispatch;
mod events;
mod forms;
mod policy;
mod projection;
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, profile::ProfileId, state::AppState};
use inputforge_gui_dx::profiles::{Bindings, Intent, View};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Scope {
    profile: Option<ProfileId>,
    path: Option<std::path::PathBuf>,
}
impl Scope {
    fn from_state(s: &AppState) -> Self {
        Self {
            profile: s.active_profile.as_ref().map(|p| p.id().clone()),
            path: s.profile_path.clone(),
        }
    }
}
#[derive(Debug, Clone)]
struct Session {
    owner: u64,
    picker: u64,
    view: Signal<View>,
    scope: Scope,
}
/// Captured native picker ownership, invalidated by panel or profile replacement.
#[derive(Debug, Clone)]
pub struct Picker {
    owner: u64,
    serial: u64,
    scope: Scope,
}
/// Per-window profile callbacks. Mounted owners reject obsolete picker results.
#[derive(Debug, Clone)]
pub struct Controller {
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    session: Rc<RefCell<Option<Session>>>,
    serial: Rc<RefCell<u64>>,
}
impl Controller {
    /// Use the existing engine transport and coherent engine state.
    #[must_use]
    pub fn new(state: Arc<RwLock<AppState>>, commands: mpsc::Sender<EngineCommand>) -> Self {
        Self {
            state,
            commands,
            session: Rc::default(),
            serial: Rc::default(),
        }
    }
    /// Publish a fresh panel session, invalidating a previous mount.
    #[must_use]
    pub fn mount(&self, view: Signal<View>) -> u64 {
        *self.serial.borrow_mut() += 1;
        let owner = *self.serial.borrow();
        *self.session.borrow_mut() = Some(Session {
            owner,
            picker: 0,
            view,
            scope: self
                .state
                .try_read()
                .map(|s| Scope::from_state(&s))
                .unwrap_or_default(),
        });
        self.refresh(owner);
        owner
    }
    /// Dispose only the captured panel, leaving newer mounts intact.
    pub fn unmount(&self, owner: u64) {
        if self
            .session
            .borrow()
            .as_ref()
            .is_some_and(|s| s.owner == owner)
        {
            self.session.borrow_mut().take();
        }
    }
    /// Supply the profile panel's host callbacks.
    #[must_use]
    pub fn bindings(&self) -> Bindings {
        let mount = self.clone();
        let dispatch = self.clone();
        let unmount = self.clone();
        let reconcile = self.clone();
        let key = self.clone();
        let focus = self.clone();
        Bindings {
            mount: Callback::new(move |v| mount.mount(v)),
            dispatch: Callback::new(move |(o, w, i)| dispatch.dispatch_captured(o, w, i)),
            reconcile: Callback::new(move |meta| reconcile.reconcile(&meta)),
            unmount: Callback::new(move |o| unmount.unmount(o)),
            rename_key: Callback::new(move |(o, w, e)| key.rename_key(o, w, &e)),
            focus: Callback::new(move |(o, w, e)| focus.focus(o, w, e)),
            open_panel: Callback::new(|mut panel: Signal<_>| {
                panel.set(inputforge_gui_dx::PanelSlot::Profiles);
            }),
        }
    }
    /// Interpret input against its captured owner and current engine state.
    /// Returns false on rejection so a menu retains its focus and open state.
    #[must_use]
    pub fn dispatch(&self, owner: u64, intent: Intent) -> bool {
        if intent == Intent::Pick {
            return self.pick(owner);
        }
        let Some(state) = self.state.try_read() else {
            self.error(owner, "Profiles are busy. Try again.".into());
            return false;
        };
        let mut sessions = self.session.borrow_mut();
        let Some(session) = sessions.as_mut().filter(|s| s.owner == owner) else {
            return false;
        };
        let mut next = session.clone();
        let mut view = session.view.peek().clone();
        let result = dispatch::prepare(&mut next, &mut view, &state, intent);
        policy::project(&state, &mut view);
        drop(state);
        let result = result
            .and_then(|command| command.map_or(Ok(()), |cmd| dispatch::send(&self.commands, cmd)));
        match result {
            Ok(()) => {
                view.error = None;
                *session = next;
                session.view.set(view);
                true
            }
            Err(error) => {
                session.view.write().error = Some(error);
                false
            }
        }
    }
    fn dispatch_captured(&self, owner: u64, workflow: u64, intent: Intent) -> bool {
        let current = self
            .session
            .borrow()
            .as_ref()
            .is_some_and(|s| s.owner == owner && s.view.peek().workflow == workflow);
        current && self.dispatch(owner, intent)
    }
    fn error(&self, owner: u64, error: String) {
        if let Some(session) = self
            .session
            .borrow_mut()
            .as_mut()
            .filter(|s| s.owner == owner)
        {
            session.view.write().error = Some(error);
        }
    }
}
