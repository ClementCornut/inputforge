//! Mounted sortable owner identity and synchronous transport cleanup.
use dioxus::prelude::*;
use inputforge_gui_dx::sortable::{Bindings, State};
pub(super) fn clear<G: 'static + Clone + PartialEq>(mut state: State<G>) {
    state.drag_from.set(None);
    state.drag_group.set(None);
    state.handle_id.set(None);
    if state.drop_target.peek().is_some() {
        state.drop_target.set(None);
    }
}
pub(super) fn cancel(field: &str, key: &str) {
    let key = format!("{key:?}");
    let _ = document::eval(&format!(
        "if (window.__ifSortableDrag?.{field} === {key}) window.__ifSortableDrag.cancel();"
    ));
}
pub(super) fn use_state<G: 'static + Clone + PartialEq>(bindings: Bindings<G>) -> State<G> {
    let mut drag_from = use_signal(|| None);
    let mut drag_group = use_signal(|| None);
    let mut drop_target = use_signal(|| None);
    let live_announcement = use_signal(String::new);
    let mut handle_id = use_signal(|| None);
    let mut closed = use_signal(|| false);
    let owner = format!("{:?}", drag_from.id());
    use_hook(|| {
        let _ = document::eval(include_str!("../../../assets/sortable_drag.js"));
    });
    let close = use_callback(move |()| {
        if !*closed.peek() {
            closed.set(true);
            cancel("owner", &owner);
            drag_from.set(None);
            drag_group.set(None);
            drop_target.set(None);
            handle_id.set(None);
        }
    });
    use_drop(move || close.call(()));
    State {
        drag_from,
        drag_group,
        drop_target,
        live_announcement,
        handle_id,
        closed,
        close,
        bindings,
    }
}
