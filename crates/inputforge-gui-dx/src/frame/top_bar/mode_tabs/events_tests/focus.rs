//! Failed real mode operations retain both the menu and native document focus.
use super::fixture::*;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct RecordingDocument(RefCell<Vec<String>>);
impl document::Document for RecordingDocument {
    fn eval(&self, js: String) -> document::Eval {
        self.0.borrow_mut().push(js);
        document::Document::eval(&document::NoOpDocument, String::new())
    }
}

#[test]
fn adoption_mode_rejected_menu_delivery_does_not_restore_dom_focus() {
    for label in ["Activate", "Set as default"] {
        let (mut dom, _, receiver) = mount();
        let doc = Rc::new(RecordingDocument::default());
        dom.provide_root_context(Rc::clone(&doc) as Rc<dyn document::Document>);
        menu(&mut dom);
        let before = doc.0.borrow().len();
        drop(receiver);
        click(&dom, label);
        settle(&mut dom);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("role=\"menu\""));
        assert!(html.contains("role=\"alert\""));
        assert!(
            !doc.0.borrow()[before..]
                .iter()
                .any(|script| script.contains("?.focus();")),
            "failed {label} requested native focus outside its retained menu"
        );
    }
}
