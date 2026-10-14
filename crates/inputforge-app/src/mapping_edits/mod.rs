//! Application-owned mapping edits, restoration and editor-scoped history.
mod dispatch;
mod grouping;
mod history;
mod paths;
mod prepare;
use dioxus::prelude::{Callback, ReadableExt, WritableExt};
use inputforge_core::{engine::EngineCommand, state::AppState};
use inputforge_gui_dx::{
    mapping_edits::{Bindings, Publication, Summary, View},
    snapshots::MappingKey,
};
use parking_lot::RwLock;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, mpsc::Sender},
    time::Instant,
};
/// One mounted editor's mapping controller and private restore snapshots.
#[derive(Debug, Clone)]
pub struct Controller {
    state: Arc<RwLock<AppState>>,
    commands: Sender<EngineCommand>,
    inner: Rc<RefCell<State>>,
    owner: u128,
    serial: Rc<Cell<u64>>,
}
#[derive(Debug, Default)]
struct State {
    history: history::History,
    publication: Option<Publication>,
    navigation: Option<(String, Option<MappingKey>)>,
}
impl Controller {
    /// Create an editor-scoped controller without modifying engine state.
    #[must_use]
    pub fn new(state: Arc<RwLock<AppState>>, commands: Sender<EngineCommand>) -> Self {
        Self {
            state,
            commands,
            inner: Rc::default(),
            owner: uuid::Uuid::new_v4().as_u128(),
            serial: Rc::default(),
        }
    }
    /// Supply the renderer's edit, history and lifecycle callbacks.
    #[must_use]
    pub fn bindings(&self) -> Bindings {
        let install = self.clone();
        let dispatch = self.clone();
        let undo = self.clone();
        let redo = self.clone();
        let forget = self.clone();
        let group = self.clone();
        let reconcile = self.clone();
        let dispose = self.clone();
        Bindings {
            install: Callback::new(move |publication| {
                install.inner.borrow_mut().publication = Some(publication);
                install.reconcile();
            }),
            dispatch: Callback::new(move |request| dispatch.dispatch(request, Instant::now())),
            undo: Callback::new(move |key| undo.undo(&key)),
            redo: Callback::new(move |key| redo.redo(&key)),
            forget: Callback::new(move |key| {
                forget.inner.borrow_mut().history.forget(&key);
                forget.publish();
            }),
            end_group: Callback::new(move |()| group.end_group()),
            reconcile: Callback::new(move |()| reconcile.reconcile()),
            dispose: Callback::new(move |()| {
                *dispose.inner.borrow_mut() = State::default();
            }),
        }
    }
    /// Restore the previous mapping state after successful delivery.
    #[must_use]
    pub fn undo(&self, key: &MappingKey) -> bool {
        self.restore(key, false)
    }
    /// Restore the final delivered state of an undone edit or burst.
    #[must_use]
    pub fn redo(&self, key: &MappingKey) -> bool {
        self.restore(key, true)
    }
    /// Prepare labels without exposing mutable restore snapshots.
    #[must_use]
    pub fn view(&self) -> View {
        prepare_view(&self.inner.borrow().history)
    }
    /// End the current nudge group on focus change or intervening operation.
    pub fn end_group(&self) {
        self.inner.borrow_mut().history.burst = None;
    }
    fn reconcile(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            if let Some(p) = inner.publication {
                let navigation = (p.mode.peek().clone(), p.selection.peek().clone());
                if inner.navigation.as_ref() != Some(&navigation) {
                    inner.history.burst = None;
                    inner.navigation = Some(navigation);
                }
            }
            if let Some(state) = self.state.try_read() {
                inner.history.reconcile(&state, self.owner);
            }
        };
        self.publish();
    }
    fn fence(&self) -> Option<u64> {
        let serial = self
            .serial
            .get()
            .checked_add(1)
            .expect("mapping edit serial exhausted");
        self.commands
            .send(EngineCommand::MappingEditBarrier {
                owner: self.owner,
                serial,
            })
            .ok()?;
        self.serial.set(serial);
        Some(serial)
    }
    fn publish(&self) {
        let (publication, view) = {
            let inner = self.inner.borrow();
            (inner.publication, prepare_view(&inner.history))
        };
        if let Some(mut signal) = publication.map(|p| p.history)
            && signal.try_peek().is_ok_and(|old| *old != view)
            && let Ok(mut writer) = signal.try_write()
        {
            *writer = view;
        }
    }
}

fn prepare_view(history: &history::History) -> View {
    View {
        stacks: history
            .stacks
            .iter()
            .map(|(key, s)| {
                (
                    key.clone(),
                    Summary {
                        undo: s.undo.iter().map(|e| e.label.clone()).collect(),
                        redo: s.redo.iter().map(|e| e.label.clone()).collect(),
                    },
                )
            })
            .collect(),
    }
}
