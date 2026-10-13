//! Editor-scoped Escape interception with synchronous listener cleanup.
use super::Controller;
use dioxus::prelude::*;
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
    pub(super) fn close(self) {
        // Dispatch directly: a task spawned during scope teardown would be cancelled.
        let _ = document::eval(&format!(
            "window['{}']?.(); delete window['{}'];",
            self.key, self.key
        ));
        let _ = self.handle.send("__shutdown__");
    }
}
pub(super) fn listen(controller: Controller, serial: u64) -> Listener {
    let key = format!("if-live-capture-{}", uuid::Uuid::new_v4().simple());
    let mut handle = document::eval(&format!(
        r"
        const h = (ev) => {{
            if (ev.key === 'Escape') {{ ev.stopPropagation(); dioxus.send('esc'); }}
        }};
        window.addEventListener('keydown', h, true);
        window['{key}'] = () => window.removeEventListener('keydown', h, true);
        (async () => {{
            while (await dioxus.recv() !== '__shutdown__') {{}}
            window['{key}']?.(); delete window['{key}'];
            dioxus.send('shutdown_ack');
        }})();
    "
    ));
    let listener = Listener { handle, key };
    spawn(async move {
        if let Ok(message) = handle.recv::<String>().await
            && message == "esc"
        {
            controller.cancel_serial(serial);
        }
    });
    listener
}
