use super::{ClickAwayListener, document};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
fn mount() -> Element {
    inputforge_app::ui::use_provider();
    let shown = use_context::<Rc<Cell<bool>>>();
    let count = use_context::<Rc<Cell<usize>>>();
    rsx! { if shown.get() { ClickAwayListener {target_id:"target".to_owned(),exclude_id:Some("trigger".to_owned()),on_click_away:move |()| count.set(count.get()+1),div{id:"target","Child"}} } }
}
#[test]
fn click_away_disposes_synchronously_once_and_rejects_obsolete_owners() {
    let shown = Rc::new(Cell::new(true));
    let count = Rc::new(Cell::new(0_usize));
    let doc = Rc::new(document::Document::default());
    let mut dom = VirtualDom::new(mount);
    dom.provide_root_context(Rc::clone(&shown));
    dom.provide_root_context(Rc::clone(&count));
    dom.provide_root_context(Rc::clone(&doc) as Rc<dyn dioxus::document::Document>);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    assert_eq!(doc.listeners.borrow().len(), 1);
    doc.event(0, serde_json::json!("away"));
    dom.render_immediate_to_vec();
    assert_eq!(count.get(), 1);
    doc.event(0, serde_json::json!("away"));
    shown.set(false);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate_to_vec();
    assert_eq!(doc.cleanups(), 1);
    assert_eq!(doc.shutdowns(), 1);
    let before = count.get();
    shown.set(true);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate_to_vec();
    dom.render_immediate_to_vec();
    assert_eq!(doc.listeners.borrow().len(), 2);
    doc.event(0, serde_json::json!("away"));
    dom.render_immediate_to_vec();
    assert_eq!(count.get(), before);
    doc.event(1, serde_json::json!("away"));
    dom.render_immediate_to_vec();
    assert_eq!(count.get(), before + 1);
    drop(dom);
    assert_eq!(doc.cleanups(), 2);
    assert_eq!(doc.shutdowns(), 2);
    assert!(doc.scripts().iter().any(|s| s.contains("composedPath")));
}
fn dialog_mount() -> Element {
    inputforge_app::ui::use_provider();
    let shown = use_context::<Rc<Cell<bool>>>();
    let open = use_signal(|| true);
    rsx! { if shown.get() { super::DialogRoot {open,onclose:move |()| true,super::DialogTitle {"Title"}} } }
}
#[test]
fn dialog_cancel_listener_is_cleaned_before_each_remount() {
    let shown = Rc::new(Cell::new(true));
    let doc = Rc::new(document::Document::default());
    let mut dom = VirtualDom::new(dialog_mount);
    dom.provide_root_context(Rc::clone(&shown));
    dom.provide_root_context(Rc::clone(&doc) as Rc<dyn dioxus::document::Document>);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    assert_eq!(
        doc.scripts()
            .iter()
            .filter(|s| s.contains("addEventListener('cancel'"))
            .count(),
        1
    );
    shown.set(false);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate_to_vec();
    assert_eq!(doc.cleanups(), 1);
    shown.set(true);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate_to_vec();
    dom.render_immediate_to_vec();
    assert_eq!(
        doc.scripts()
            .iter()
            .filter(|s| s.contains("addEventListener('cancel'"))
            .count(),
        2
    );
    drop(dom);
    assert_eq!(doc.cleanups(), 2);
}
