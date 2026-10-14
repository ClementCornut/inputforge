//! Browser gesture installation and cancellation for each mounted owner/handle.
use dioxus::prelude::*;

pub(super) fn use_drag(owner: &str) {
    use_hook(|| {
        let _ = document::eval(include_str!("../../../assets/components/sortable_drag.js"));
    });
    use_cancel("owner", owner);
}

pub(super) fn use_handle() -> String {
    let identity = use_signal(|| ());
    let key = format!("{:?}", identity.id());
    use_cancel("handle", &key);
    key
}

fn use_cancel(field: &'static str, key: &str) {
    let key = serde_json::Value::String(key.to_owned());
    use_drop(move || {
        let _ = document::eval(&format!(
            "if (window.__ifSortableDrag?.{field} === {key}) window.__ifSortableDrag.cancel();"
        ));
    });
}
