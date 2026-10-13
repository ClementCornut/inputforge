//! Application-owned live input capture and consumer delivery.
mod confirmation;
mod delivery;
#[cfg(test)]
#[path = "tests/detection.rs"]
mod detection_tests;
mod escape;
mod machine;
mod policy;
mod sample;

use dioxus::prelude::*;
use inputforge_core::{
    engine::EngineCommand,
    profile::controllers::ControlLayout,
    state::AppState,
    types::{DeviceId, InputAddress},
};
use inputforge_gui_dx::capture::{Bindings, Publication, Request, Target};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

/// One editor's live-capture controller.
#[derive(Debug, Clone)]
pub struct Controller {
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    inner: Rc<RefCell<State>>,
}
#[derive(Debug, Default)]
struct State {
    publication: Option<Publication>,
    armed: Option<Armed>,
    serial: u64,
    listener: Option<escape::Listener>,
}
#[derive(Debug)]
struct Armed {
    request: Request,
    target: Target,
    core: machine::CoreState,
    generation: Option<u64>,
    layouts: Vec<(DeviceId, ControlLayout)>,
    candidate: Option<InputAddress>,
    cancelled: bool,
}
impl Controller {
    /// Create an editor-scoped capture controller.
    #[must_use]
    pub fn new(state: Arc<RwLock<AppState>>, commands: mpsc::Sender<EngineCommand>) -> Self {
        Self {
            state,
            commands,
            inner: Rc::default(),
        }
    }
    /// Supply callbacks for the renderer's root and consumers.
    #[must_use]
    pub fn bindings(&self) -> Bindings {
        let install = self.clone();
        let start = self.clone();
        let cancel = self.clone();
        let dispose = self.clone();
        Bindings {
            install: Callback::new(move |active| {
                install.inner.borrow_mut().publication = Some(active);
            }),
            start: Callback::new(move |request| start.start(request)),
            cancel: Callback::new(move |owner| cancel.cancel(owner)),
            dispose: Callback::new(move |()| {
                dispose.finish(false, false);
                dispose.inner.borrow_mut().publication = None;
            }),
        }
    }
    fn start(&self, request: Request) {
        let replaced = self.finish(false, false);
        let target = request.target.peek().clone();
        let serial = {
            let mut inner = self.inner.borrow_mut();
            inner.serial = inner.serial.wrapping_add(1);
            inner.armed = Some(Armed {
                target,
                request,
                core: machine::CoreState {
                    filter: request.filter,
                    ..Default::default()
                },
                generation: None,
                layouts: vec![],
                candidate: None,
                cancelled: false,
            });
            inner.serial
        };
        set(request.listening, true);
        if let Some(active) = self.inner.borrow().publication {
            set(active.active, true);
        }
        let listener = escape::listen(self.clone(), serial);
        self.inner.borrow_mut().listener = Some(listener);
        if let Some(previous) = replaced
            && previous.owner != request.owner
        {
            previous.on_cancel.call(());
        }
    }
    fn cancel(&self, owner: ScopeId) {
        let owns_capture = self
            .inner
            .borrow()
            .armed
            .as_ref()
            .is_some_and(|a| a.request.owner == owner);
        if owns_capture {
            self.finish(false, true);
        }
    }
    fn cancel_serial(&self, serial: u64) {
        if self.inner.borrow().serial == serial {
            self.finish(false, true);
        }
    }
    fn finish(&self, delivered: bool, notify: bool) -> Option<Request> {
        let (armed, listener, active) = {
            let mut inner = self.inner.borrow_mut();
            (inner.armed.take(), inner.listener.take(), inner.publication)
        };
        if let Some(listener) = listener {
            listener.close();
        }
        if let Some(active) = active {
            set(active.active, false);
        }
        let request = armed?.request;
        set(request.listening, false);
        if !delivered && notify {
            request.on_cancel.call(());
        }
        Some(request)
    }
}
fn set(mut signal: Signal<bool>, value: bool) {
    if let Ok(mut current) = signal.try_write() {
        *current = value;
    }
}
