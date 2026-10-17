use super::event;
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
struct Log {
    open: RefCell<Option<Signal<bool>>>,
    calls: RefCell<Vec<&'static str>>,
    accept: Cell<bool>,
}
fn dirty() -> Element {
    inputforge_app::ui::use_provider();
    let open = use_signal(|| true);
    let log = use_context::<Rc<Log>>();
    *log.open.borrow_mut() = Some(open);
    let cancel = Rc::clone(&log);
    let discard = Rc::clone(&log);
    let save = log;
    rsx! { inputforge_gui_dx::patterns::DirtyConfirmDialog {open,
        oncancel:move |()| {cancel.calls.borrow_mut().push("cancel");cancel.accept.get()},
        ondiscard:move |()| {discard.calls.borrow_mut().push("discard");discard.accept.get()},
        onsave:move |()| {save.calls.borrow_mut().push("save");save.accept.get()}
    }}
}
#[test]
fn dirty_resolution_retains_failed_actions_and_rejects_late_closed_callbacks() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let log = Rc::new(Log::default());
    let mut dom = VirtualDom::new(dirty);
    dom.provide_root_context(Rc::clone(&log));
    dom.rebuild_in_place();
    for index in [4, 3, 2] {
        event(&dom, "click", index, SerializedMouseData::default());
        assert!(log.open.borrow().unwrap()());
    }
    assert_eq!(*log.calls.borrow(), ["save", "discard", "cancel"]);
    log.accept.set(true);
    event(&dom, "click", 4, SerializedMouseData::default());
    assert!(!log.open.borrow().unwrap()());
    event(&dom, "click", 4, SerializedMouseData::default());
    assert_eq!(
        log.calls.borrow().len(),
        4,
        "a closed dialog must not submit again"
    );
}
