use super::{DialogBody, DialogDescription, DialogRoot, DialogTitle, event};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, Copy)]
struct Values {
    open: Signal<bool>,
    dismissible: Signal<bool>,
    backdrop: Signal<bool>,
}
#[derive(Default)]
struct DialogState {
    values: RefCell<Option<Values>>,
    accept: Cell<bool>,
    calls: Cell<usize>,
}
fn dialog() -> Element {
    inputforge_app::ui::use_provider();
    let state = use_context::<Rc<DialogState>>();
    let open = use_signal(|| true);
    let dismissible = use_signal(|| true);
    let backdrop = use_signal(|| false);
    *state.values.borrow_mut() = Some(Values {
        open,
        dismissible,
        backdrop,
    });
    rsx! {DialogRoot {open,dismissible:dismissible(),close_on_backdrop_click:backdrop(),onclose:move |()| {state.calls.set(state.calls.get()+1);state.accept.get()},
        DialogTitle {"Title"} DialogDescription {"Description"} DialogBody {"Body"}
    }}
}
#[test]
fn dialog_dismissal_uses_current_policy_and_acknowledgement() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let state = Rc::new(DialogState::default());
    let mut dom = VirtualDom::new(dialog);
    dom.provide_root_context(Rc::clone(&state));
    dom.rebuild_in_place();
    let mut values = state.values.borrow().unwrap();
    event(&dom, "click", 0, SerializedMouseData::default());
    assert_eq!(state.calls.get(), 0);
    let cancel = event(&dom, "cancel", 0, SerializedCancelData {});
    assert!(!cancel.default_action_enabled());
    assert!(!cancel.propagates());
    assert!((values.open)());
    assert_eq!(state.calls.get(), 1);
    state.accept.set(true);
    values.dismissible.set(false);
    dom.render_immediate_to_vec();
    event(&dom, "click", 0, SerializedMouseData::default());
    event(&dom, "cancel", 0, SerializedCancelData {});
    assert_eq!(state.calls.get(), 1);
    values.backdrop.set(true);
    dom.render_immediate_to_vec();
    event(&dom, "click", 0, SerializedMouseData::default());
    event(&dom, "cancel", 0, SerializedCancelData {});
    assert_eq!(state.calls.get(), 1);
    assert!((values.open)());
    values.dismissible.set(true);
    dom.render_immediate_to_vec();
    event(&dom, "click", 1, SerializedMouseData::default());
    assert!((values.open)(), "panel clicks must stay inside");
    event(&dom, "click", 0, SerializedMouseData::default());
    assert!(!(values.open)());
    assert_eq!(state.calls.get(), 2);
    event(&dom, "cancel", 0, SerializedCancelData {});
    assert_eq!(state.calls.get(), 2, "closed dialogs ignore late dismissal");
    let html = dioxus_ssr::render(&dom);
    for prefix in ["aria-labelledby=\"", "aria-describedby=\""] {
        let id = html
            .split(prefix)
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert!(html.contains(&format!("id=\"{id}\"")));
    }
}
