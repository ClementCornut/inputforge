//! App-owned device selection, alias drafts, guarded removal and clipboard writes.
mod clipboard;
mod dispatch;
mod policy;
use dioxus::prelude::*;
use inputforge_core::{
    engine::{DeviceTarget, EngineCommand},
    state::AppState,
};
use inputforge_gui_dx::controllers::inspector::{Bindings, Intent, View};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
#[derive(Debug, Clone)]
struct Session {
    owner: u64,
    view: Signal<View>,
    target: Option<DeviceTarget>,
    catalog: Vec<DeviceTarget>,
}
/// Shared inspector lifecycle and native clipboard ownership for one editor.
#[derive(Debug, Clone)]
pub struct Controller {
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    serial: Rc<RefCell<u64>>,
    session: Rc<RefCell<Option<Session>>>,
    clipboard: Rc<RefCell<clipboard::Clipboard>>,
}
impl Controller {
    /// Bind inspector actions to current engine state and the existing command sender.
    #[must_use]
    pub fn new(state: Arc<RwLock<AppState>>, commands: mpsc::Sender<EngineCommand>) -> Self {
        Self {
            state,
            commands,
            serial: Rc::default(),
            session: Rc::default(),
            clipboard: Rc::default(),
        }
    }
    /// Supply mounted callbacks without exposing engine or native service access.
    #[must_use]
    pub fn bindings(&self) -> Bindings {
        let mount = self.clone();
        let dispatch = self.clone();
        let refresh = self.clone();
        let unmount = self.clone();
        let keys = self.clone();
        Bindings {
            mount: Callback::new(move |view| mount.mount(view)),
            dispatch: Callback::new(move |(o, w, i)| dispatch.dispatch(o, w, i)),
            key: Callback::new(move |(o, w, key): (_, _, String)| {
                let intent = match key.as_str() {
                    "Enter" => Intent::Save,
                    "Escape" => Intent::Cancel,
                    _ => return false,
                };
                let _ = keys.dispatch(o, w, intent);
                true
            }),
            reconcile: Callback::new(move |()| refresh.refresh()),
            unmount: Callback::new(move |o| unmount.unmount(o)),
        }
    }
    fn mount(&self, view: Signal<View>) -> u64 {
        *self.serial.borrow_mut() += 1;
        let owner = *self.serial.borrow();
        *self.session.borrow_mut() = Some(Session {
            owner,
            view,
            target: None,
            catalog: vec![],
        });
        self.refresh();
        owner
    }
    fn unmount(&self, owner: u64) {
        let mut s = self.session.borrow_mut();
        if s.as_ref().is_some_and(|s| s.owner == owner) {
            s.take();
        }
    }
    /// Reconcile on the existing bridge cadence, skipping contended engine reads.
    pub fn refresh(&self) {
        let Some(state) = self.state.try_read() else {
            return;
        };
        if let Some(s) = self.session.borrow_mut().as_mut() {
            refresh(s, &state);
        }
    }
}
fn refresh(s: &mut Session, state: &AppState) {
    let rows = crate::bridge::devices::build_device_panel_rows(state);
    let mut next = s.view.peek().clone();
    let selected = next
        .selected
        .as_ref()
        .and_then(|r| rows.iter().find(|row| row.device_id == r.device_id));
    let catalog: Vec<_> = rows
        .iter()
        .filter_map(|r| DeviceTarget::capture(state, &r.device_id))
        .collect();
    if s.catalog != catalog {
        s.catalog = catalog;
        next.workflow += 1;
    }
    if selected.is_none() && (next.selected.is_some() || !rows.is_empty()) {
        next.selected = policy::first(&rows).cloned();
        next.draft = next
            .selected
            .as_ref()
            .map_or_else(String::new, |r| r.alias.clone());
        s.target = next
            .selected
            .as_ref()
            .and_then(|r| DeviceTarget::capture(state, &r.device_id));
        next.error = None;
        next.workflow += 1;
    } else if s.target.as_ref().is_some_and(|t| t.matches(state)) {
        next.selected = selected.cloned();
    }
    next.rows = rows;
    policy::availability(&mut next, state);
    if *s.view.peek() != next {
        s.view.set(next);
    }
}
#[cfg(test)]
mod tests;
