//! Idle add-pad dismissal preserves the existing capture owner and filter escape.
use super::listener::stop;
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
pub(super) fn use_idle_escape(active: ReadSignal<bool>, callback: EventHandler<()>) {
    let dismiss = use_callback(move |()| {
        if !*active.peek() {
            callback.call(());
        }
    });
    let cleanup = use_hook(|| {
        let key = format!("if-rail-add-{}", uuid::Uuid::new_v4().simple());
        let mut handle = document::eval(
            &include_str!("../../../assets/rail_add_escape.js").replace("__LISTENER_KEY__", &key),
        );
        let closed = Rc::new(Cell::new(false));
        let cleanup = stop(handle, key, Rc::clone(&closed));
        spawn(async move {
            while let Ok(message) = handle.recv::<String>().await {
                if closed.get() {
                    break;
                }
                if message == "esc" {
                    dismiss.call(());
                }
            }
        });
        cleanup
    });
    use_drop(move || cleanup.call(()));
}
