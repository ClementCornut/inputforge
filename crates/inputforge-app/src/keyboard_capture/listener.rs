//! Editor-scoped keyboard interception with synchronous listener cleanup.
use super::{Controller, translation::BrowserKeyboardPayload};
use dioxus::prelude::{document, spawn};
use serde::Deserialize;
use std::fmt;
#[derive(Deserialize)]
struct Message {
    serial: u64,
    #[serde(flatten)]
    payload: BrowserKeyboardPayload,
}
pub(super) struct Listener {
    handle: document::Eval,
    pub(super) key: String,
}
impl fmt::Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Listener")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}
impl Listener {
    pub(super) fn arm(&self, serial: u64) {
        let _ = document::eval(&format!("window['{}']?.arm({serial});", self.key));
    }
    pub(super) fn disarm(&self) {
        let _ = document::eval(&format!("window['{}']?.disarm();", self.key));
    }
    pub(super) fn close(self) {
        // A task spawned during scope teardown would be cancelled before cleanup.
        let _ = document::eval(&format!(
            "window['{}']?.(); delete window['{}'];",
            self.key, self.key
        ));
        let _ = self.handle.send("__shutdown__");
    }
}
pub(super) fn listen(controller: Controller) -> Listener {
    let key = format!("if-keyboard-capture-{}", uuid::Uuid::new_v4().simple());
    let script =
        include_str!("../../assets/keyboard_capture_listener.js").replace("__LISTENER_KEY__", &key);
    let mut handle = document::eval(&script);
    let listener = Listener {
        handle,
        key: key.clone(),
    };
    spawn(async move {
        while let Ok(message) = handle.recv::<Message>().await {
            controller.payload(message.serial, &message.payload);
        }
        let current = controller
            .inner
            .borrow()
            .listener
            .as_ref()
            .is_some_and(|listener| listener.key == key);
        if current {
            controller.dispose();
        }
    });
    listener
}
