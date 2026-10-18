//! The ready transition must not focus until the renderer applies visible styles.
use dioxus::prelude::*;
use inputforge_gui_dx::overlays::FocusAction;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct RecordingDocument(RefCell<Vec<String>>);
impl document::Document for RecordingDocument {
    fn eval(&self, js: String) -> document::Eval {
        self.0.borrow_mut().push(js);
        document::Document::eval(&document::NoOpDocument, String::new())
    }
}
#[derive(Default)]
struct Readiness(RefCell<Option<Signal<bool>>>);
fn fixture() -> Element {
    let ready = use_signal(|| false);
    *use_context::<Rc<Readiness>>().0.borrow_mut() = Some(ready);
    super::use_focus(true, ready(), "mapping-menu".into());
    rsx! { div { id: "mapping-menu", style: if ready() { "visibility: visible" } else { "visibility: hidden" }, button { "Rename" } } }
}
fn settle(dom: &mut VirtualDom) {
    for _ in 0..3 {
        dom.render_immediate_to_vec();
    }
}
#[test]
fn regression_menu_initial_focus_waits_for_visibility_patch_but_navigation_is_immediate() {
    let document = Rc::new(RecordingDocument::default());
    let readiness = Rc::new(Readiness::default());
    let mut dom = VirtualDom::new(fixture);
    dom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    dom.provide_root_context(Rc::clone(&readiness));
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(
        document.0.borrow().is_empty(),
        "hidden menus cannot take initial focus"
    );
    readiness.0.borrow().unwrap().set(true);
    settle(&mut dom);
    let scripts = document.0.borrow();
    assert_eq!(
        scripts.len(),
        1,
        "the ready transition schedules focus once"
    );
    assert!(
        scripts[0].starts_with("requestAnimationFrame("),
        "initial focus must run after the visible style reaches the browser: {}",
        scripts[0]
    );
    drop(scripts);
    dom.in_scope(ScopeId::APP, || {
        super::focus("mapping-menu", FocusAction::Next);
    });
    let scripts = document.0.borrow();
    assert_eq!(scripts.len(), 2);
    assert!(!scripts[1].starts_with("requestAnimationFrame("));
    assert!(scripts[1].ends_with("(\"mapping-menu\", \"down\");"));
}
