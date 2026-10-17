use super::document;
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
struct Log {
    active: RefCell<Option<Signal<bool>>>,
    visible: Cell<bool>,
    calls: Cell<usize>,
}
fn root() -> Element {
    inputforge_app::ui::use_provider();
    let log = use_context::<Rc<Log>>();
    rsx! { if log.visible.get() {Idle{}} }
}
#[component]
fn Idle() -> Element {
    let active = use_signal(|| false);
    let log = use_context::<Rc<Log>>();
    *log.active.borrow_mut() = Some(active);
    let callback = use_callback(move |()| log.calls.set(log.calls.get() + 1));
    let bindings = use_context::<inputforge_gui_dx::controls::Bindings>();
    (bindings.overlays.idle_escape)(active.into(), callback);
    rsx! {div {"Draft"}}
}
#[test]
fn add_dismissal_observes_capture_and_disposes_before_remount() {
    let log = Rc::new(Log::default());
    log.visible.set(true);
    let doc = Rc::new(document::Document::default());
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&log));
    dom.provide_root_context(Rc::clone(&doc) as Rc<dyn dioxus::document::Document>);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    doc.event(0, serde_json::json!("esc"));
    dom.render_immediate_to_vec();
    assert_eq!(log.calls.get(), 1);
    log.active.borrow().unwrap().set(true);
    doc.event(0, serde_json::json!("esc"));
    dom.render_immediate_to_vec();
    assert_eq!(log.calls.get(), 1);
    log.visible.set(false);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate_to_vec();
    assert_eq!(doc.shutdowns(), 1);
    assert_eq!(doc.cleanups(), 1);
    log.visible.set(true);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate_to_vec();
    dom.render_immediate_to_vec();
    doc.event(0, serde_json::json!("esc"));
    dom.render_immediate_to_vec();
    assert_eq!(log.calls.get(), 1);
    doc.event(1, serde_json::json!("esc"));
    dom.render_immediate_to_vec();
    assert_eq!(log.calls.get(), 2);
    drop(dom);
    assert_eq!(doc.shutdowns(), 2);
    assert_eq!(doc.cleanups(), 2);
}
