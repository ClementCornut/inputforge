//! App-owned browser listener with synchronous removal during scope teardown.
use super::Controller;
use dioxus::prelude::{document, spawn};
use std::fmt;
pub(super) struct Listener {
    handle: document::Eval,
    key: String,
}
impl fmt::Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Listener")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}
impl Listener {
    pub(super) fn arm(&self, generation: u64) {
        let _ = document::eval(&format!("window['{}']?.arm({generation});", self.key));
    }
    pub(super) fn close(self) {
        let _ = document::eval(&format!(
            "window['{}']?.(); delete window['{}'];",
            self.key, self.key
        ));
        let _ = self.handle.send("__shutdown__");
    }
}
pub(super) fn listen(controller: Controller, owner: u64) -> Listener {
    let key = format!("if-snapshot-shortcut-{}", uuid::Uuid::new_v4().simple());
    let script = include_str!("../../../assets/snapshot_shortcut.js")
        .replace("__LISTENER_KEY__", &key)
        .replace("__GENERATION__", "1");
    let mut handle = document::eval(&script);
    let listener = Listener { handle, key };
    spawn(async move {
        while let Ok(generation) = handle.recv::<u64>().await {
            controller.shortcut_payload(owner, generation);
        }
    });
    listener
}
