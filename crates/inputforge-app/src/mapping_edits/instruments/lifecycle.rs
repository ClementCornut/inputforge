//! Existing desktop eval transport with explicit per-mount listener teardown.
use super::super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::instruments::{Event, Mount, Pointer};
const SCRIPT: &str = include_str!("../../../assets/instrument_pointer.js");
pub(super) struct Listener {
    handle: document::Eval,
}
impl std::fmt::Debug for Listener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Listener").finish_non_exhaustive()
    }
}
impl Controller {
    pub(super) fn mount_instrument(&self, mount: &Mount) {
        // IDs come from the renderer's stage-path formatter; reject arbitrary script text.
        if !mount
            .plot
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return;
        }
        {
            let inner = self.inner.borrow();
            let Some(session) = inner.instruments.get(&mount.id) else {
                return;
            };
            if session.listener.is_some() {
                return;
            }
        }
        let js = SCRIPT
            .replace("__PLOT_ID__", &format!("'{}'", mount.plot))
            .replace("__OWNER__", &format!("'{}'", mount.id));
        let mut eval = document::eval(&js);
        self.inner
            .borrow_mut()
            .instruments
            .get_mut(&mount.id)
            .expect("mounted instrument exists")
            .listener = Some(Listener { handle: eval });
        let controller = self.clone();
        let mount = mount.clone();
        spawn(async move {
            while let Ok(pointer) = eval.recv::<Pointer>().await {
                if !controller
                    .inner
                    .borrow()
                    .instruments
                    .contains_key(&mount.id)
                {
                    break;
                }
                let event = Event::Pointer {
                    kind: pointer.kind.clone(),
                    observation: mount.observe.call(pointer),
                };
                if let Some(view) = super::event_instrument(&controller, mount.id, &event) {
                    mount.publish.call(view);
                } else {
                    break;
                }
            }
        });
    }
    pub(super) fn close_instrument(&self, id: u128) {
        let removed = self.inner.borrow_mut().instruments.remove(&id);
        if let Some(listener) = removed.and_then(|s| s.listener) {
            let _eval =
                document::eval(&format!("window.__inputforgeInstruments?.get('{id}')?.();"));
            let _ = listener.handle.send("__shutdown__");
        }
        self.end_group();
    }
    pub(in crate::mapping_edits) fn close_instruments(&self) {
        let ids = self
            .inner
            .borrow()
            .instruments
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for id in ids {
            self.close_instrument(id);
        }
    }
}
