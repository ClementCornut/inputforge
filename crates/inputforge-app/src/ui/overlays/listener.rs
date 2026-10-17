//! Exactly-once, synchronous scope cleanup and obsolete-callback rejection.
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub(super) fn stop(handle: document::Eval, key: String, closed: Rc<Cell<bool>>) -> Callback<()> {
    Callback::new(move |()| {
        if !closed.replace(true) {
            let _ = document::eval(&format!("window[{key:?}]?.(); delete window[{key:?}];"));
            let _ = handle.send("__shutdown__");
        }
    })
}
pub(super) fn use_click_away(target: String, exclude: Option<String>, callback: EventHandler<()>) {
    let callback = use_callback(move |()| callback.call(()));
    let cleanup = use_hook(|| Rc::new(RefCell::new(None::<Callback<()>>)));
    let unmount = Rc::clone(&cleanup);
    use_drop(move || {
        if let Some(stop) = unmount.borrow_mut().take() {
            stop.call(());
        }
    });
    use_effect(use_reactive!(|(target, exclude)| {
        if let Some(stop) = cleanup.borrow_mut().take() {
            stop.call(());
        }
        let key = format!("if-click-away-{}", uuid::Uuid::new_v4().simple());
        let script = include_str!("../../../assets/click_away.js")
            .replace("__KEY__", &format!("{key:?}"))
            .replace("__TARGET__", &format!("{target:?}"))
            .replace(
                "__EXCLUDE__",
                &exclude
                    .as_ref()
                    .map_or_else(|| "null".into(), |id| format!("{id:?}")),
            );
        let mut handle = document::eval(&script);
        let closed = Rc::new(Cell::new(false));
        *cleanup.borrow_mut() = Some(stop(handle, key, Rc::clone(&closed)));
        spawn(async move {
            while let Ok(message) = handle.recv::<String>().await {
                if closed.get() {
                    break;
                }
                if message == "away" {
                    callback.call(());
                }
            }
        });
    }));
}
