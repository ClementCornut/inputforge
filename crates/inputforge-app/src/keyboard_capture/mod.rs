//! Application-owned physical keyboard capture and editor-scoped ownership.
mod delivery;
mod listener;
mod machine;
mod policy;
#[cfg(test)]
mod tests;
mod translation;
use dioxus::prelude::{Callback, ReadableExt, ScopeId, Signal};
use inputforge_core::{state::AppState, types::KeyCombo};
use inputforge_gui_dx::keyboard_capture::{Bindings, Publication, Request, Target};
use parking_lot::RwLock;
use std::{cell::RefCell, rc::Rc, sync::Arc};

/// One editor's keyboard-capture controller.
#[derive(Debug, Clone)]
pub struct Controller {
    state: Arc<RwLock<AppState>>,
    inner: Rc<RefCell<State>>,
}
#[derive(Debug, Default)]
struct State {
    publication: Option<Publication>,
    armed: Option<Armed>,
    serial: u64,
    listener: Option<listener::Listener>,
    hint: Option<Signal<Option<&'static str>>>,
}
#[derive(Debug)]
struct Armed {
    request: Request,
    target: Target,
    capture: machine::KeyboardCapture,
    pending: Option<KeyCombo>,
}
impl Controller {
    /// Create an editor-scoped controller.
    #[must_use]
    pub fn new(state: Arc<RwLock<AppState>>) -> Self {
        Self {
            state,
            inner: Rc::default(),
        }
    }
    /// Supply renderer start, cancel, completion and lifecycle callbacks.
    #[must_use]
    pub fn bindings(&self) -> Bindings {
        let install = self.clone();
        let start = self.clone();
        let cancel = self.clone();
        let deliver = self.clone();
        let dispose = self.clone();
        Bindings {
            install: Callback::new(move |publication| install.install(publication)),
            start: Callback::new(move |request| start.start(request)),
            cancel: Callback::new(move |owner| cancel.cancel(owner)),
            deliver: Callback::new(move |owner| deliver.deliver(owner)),
            dispose: Callback::new(move |()| {
                dispose.dispose();
            }),
        }
    }
    fn install(&self, publication: Publication) {
        self.dispose();
        self.inner.borrow_mut().publication = Some(publication);
        let listener = listener::listen(self.clone());
        self.inner.borrow_mut().listener = Some(listener);
    }
    fn dispose(&self) {
        self.finish(None);
        let listener = {
            let mut inner = self.inner.borrow_mut();
            inner.publication = None;
            inner.listener.take()
        };
        if let Some(listener) = listener {
            listener.close();
        }
    }
    fn start(&self, request: Request) {
        self.finish(None);
        let Ok(target) = request.target.try_peek().map(|t| t.clone()) else {
            return;
        };
        if !self.valid(request, &target) {
            return;
        }
        let serial = {
            let mut inner = self.inner.borrow_mut();
            inner.serial = inner.serial.wrapping_add(1);
            inner.armed = Some(Armed {
                request,
                target,
                capture: machine::KeyboardCapture::default(),
                pending: None,
            });
            inner.serial
        };
        delivery::set(request.hint, None);
        delivery::set(request.pending, false);
        delivery::set(request.active, true);
        if let Some(listener) = self.inner.borrow().listener.as_ref() {
            listener.arm(serial);
        }
    }
    fn cancel(&self, owner: ScopeId) {
        if self
            .inner
            .borrow()
            .armed
            .as_ref()
            .is_some_and(|a| a.request.owner == owner)
        {
            self.finish(None);
        }
    }
    fn payload(&self, serial: u64, payload: &translation::BrowserKeyboardPayload) {
        let outcome = {
            let mut inner = self.inner.borrow_mut();
            if inner.serial != serial {
                return;
            }
            let Some(armed) = inner.armed.as_mut() else {
                return;
            };
            if armed.pending.is_some() {
                return;
            }
            armed.capture.handle_event(payload.to_capture_key_event())
        };
        match outcome {
            machine::CaptureOutcome::Continue { hint } => {
                if let Some(armed) = self.inner.borrow().armed.as_ref() {
                    delivery::set(armed.request.hint, hint);
                }
            }
            machine::CaptureOutcome::Cancel { hint } => self.finish(hint),
            machine::CaptureOutcome::Commit(combo) => {
                let request = {
                    let mut inner = self.inner.borrow_mut();
                    let armed = inner
                        .armed
                        .as_mut()
                        .expect("capture exists while processing its event");
                    armed.pending = Some(combo);
                    armed.request
                };
                if let Some(listener) = self.inner.borrow().listener.as_ref() {
                    listener.disarm();
                }
                delivery::set(request.pending, true);
            }
        }
    }
    fn finish(&self, hint: Option<&'static str>) {
        let (armed, previous_hint) = {
            let mut inner = self.inner.borrow_mut();
            (inner.armed.take(), inner.hint.take())
        };
        if let Some(previous_hint) = previous_hint {
            delivery::set(previous_hint, None);
        }
        if let Some(listener) = self.inner.borrow().listener.as_ref() {
            listener.disarm();
        }
        if let Some(armed) = armed {
            delivery::set(armed.request.pending, false);
            delivery::set(armed.request.hint, hint);
            delivery::set(armed.request.active, false);
            if hint.is_some() {
                self.inner.borrow_mut().hint = Some(armed.request.hint);
            }
        }
    }
}
