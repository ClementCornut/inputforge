//! Scope-isolated listener ownership and synchronous browser cleanup.
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
pub(super) fn close(handle: document::Eval, key: String, closed: Rc<Cell<bool>>) -> Callback<()> {
    Callback::new(move |()| {
        if !closed.replace(true) {
            let _ = document::eval(&format!("window['{key}']?.(); delete window['{key}'];"));
            let _ = handle.send("__shutdown__");
        }
    })
}
pub(super) fn key(kind: &str) -> String {
    format!("if-{kind}-{}", uuid::Uuid::new_v4().simple())
}
