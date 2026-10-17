//! Settings decisions, pending-value reconciliation and guarded prune dispatch.
mod policy;
use dioxus::prelude::*;
use inputforge_core::{
    engine::{EngineCommand, SettingsRequest},
    state::AppState,
};
use inputforge_gui_dx::{
    settings::{Bindings, Intent, View},
    snapshots::SettingsSnapshot,
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
#[derive(Debug, Clone)]
struct Session {
    owner: u64,
    identity: u128,
    view: Signal<View>,
    pending: Option<u64>,
    serial: u64,
}
/// App-owned settings callbacks sharing the existing engine command transport.
#[derive(Debug, Clone)]
pub struct Controller {
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    serial: Rc<RefCell<u64>>,
    session: Rc<RefCell<Option<Session>>>,
}
impl Controller {
    /// Bind settings to authoritative state and the existing command sender.
    #[must_use]
    pub fn new(state: Arc<RwLock<AppState>>, commands: mpsc::Sender<EngineCommand>) -> Self {
        Self {
            state,
            commands,
            serial: Rc::default(),
            session: Rc::default(),
        }
    }
    /// Publish a fresh mounted settings owner, invalidating earlier callbacks.
    #[must_use]
    pub fn mount(&self, view: Signal<View>) -> u64 {
        *self.serial.borrow_mut() += 1;
        let owner = *self.serial.borrow();
        *self.session.borrow_mut() = Some(Session {
            owner,
            identity: uuid::Uuid::new_v4().as_u128(),
            view,
            pending: None,
            serial: 0,
        });
        self.refresh();
        owner
    }
    /// Invalidate only the captured owner.
    pub fn unmount(&self, owner: u64) {
        let mut session = self.session.borrow_mut();
        if session.as_ref().is_some_and(|s| s.owner == owner) {
            session.take();
        }
    }
    /// Reconcile delivered requests on the existing editor bridge cadence.
    pub fn refresh(&self) {
        let Some(state) = self.state.try_read() else {
            return;
        };
        if let Some(session) = self.session.borrow_mut().as_mut() {
            reconcile(session, &state);
        }
    }
    /// Supply app behavior callbacks to the renderer.
    #[must_use]
    pub fn bindings(&self) -> Bindings {
        let mount = self.clone();
        let dispatch = self.clone();
        let reconcile = self.clone();
        let unmount = self.clone();
        Bindings {
            mount: Callback::new(move |v| mount.mount(v)),
            dispatch: Callback::new(move |(o, w, i)| dispatch.dispatch_captured(o, w, i)),
            reconcile: Callback::new(move |()| reconcile.refresh()),
            unmount: Callback::new(move |o| unmount.unmount(o)),
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
    /// Interpret input; failed validation/delivery retains drafts and confirmation.
    #[must_use]
    pub fn dispatch(&self, owner: u64, intent: Intent) -> bool {
        let mut sessions = self.session.borrow_mut();
        let Some(session) = sessions.as_mut().filter(|s| s.owner == owner) else {
            return false;
        };
        let buffer = matches!(&intent, Intent::Max(_) | Intent::Invalid(_));
        if intent == Intent::Cancel {
            session.view.write().prune = None;
            session.view.write().workflow += 1;
            session.view.write().error = None;
            session.view.write().buffer_error = None;
            return true;
        }
        let Some(state) = self.state.try_read() else {
            session.view.write().error = Some("Settings are busy. Try again.".into());
            return false;
        };
        reconcile(session, &state);
        let mut next = session.view.peek().clone();
        let result = policy::prepare(&mut next, &state, intent);
        drop(state);
        let result = result.and_then(|change| {
            if let Some(change) = change {
                let serial = session.serial + 1;
                self.commands
                    .send(EngineCommand::Settings(Box::new(SettingsRequest {
                        owner: session.identity,
                        serial,
                        change,
                    })))
                    .map_err(|_error| "Could not send settings change. Try again.".to_owned())?;
                session.serial = serial;
                session.pending = Some(serial);
            }
            Ok(())
        });
        match result {
            Ok(()) => {
                next.error = None;
                if buffer {
                    next.buffer_error = None;
                }
                session.view.set(next);
                true
            }
            Err(error) => {
                if buffer {
                    session.view.write().buffer_error = Some(error.clone());
                }
                session.view.write().error = Some(error);
                false
            }
        }
    }
}
fn reconcile(session: &mut Session, state: &AppState) {
    let actual = crate::bridge::settings::project(state);
    let mut view = session.view.peek().clone();
    if let Some(serial) = session.pending {
        if state.settings_ack != Some((session.identity, serial)) {
            return;
        }
        let mut expected: SettingsSnapshot = view.values.clone();
        expected.unpinned_snapshot_count = actual.unpinned_snapshot_count;
        if expected != actual {
            view.error = Some(
                "Settings change was not saved. Review the current values and try again.".into(),
            );
        }
        session.pending = None;
    }
    view.values = actual;
    if *session.view.peek() != view {
        session.view.set(view);
    }
}
