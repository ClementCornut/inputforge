//! Startup styles must exist before the first visit to a conditional panel.
use super::ThemeProvider;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct Document(RefCell<Vec<String>>);

impl document::Document for Document {
    fn eval(&self, js: String) -> document::Eval {
        document::Document::eval(&document::NoOpDocument, js)
    }

    fn create_link(&self, props: document::LinkProps) {
        if let Some(href) = props.href {
            self.0.borrow_mut().push(href);
        }
    }
}

#[test]
fn conditional_panel_styles_are_registered_before_navigation() {
    fn shell() -> Element {
        rsx! { ThemeProvider { div { "Initial mapping workspace" } } }
    }
    let document = Rc::new(Document::default());
    let mut dom = VirtualDom::new(shell);
    dom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    let links = document.0.borrow();
    for panel in ["profiles", "settings_panel"] {
        assert!(
            links.iter().any(|href| href.contains(panel)),
            "{panel} stylesheet is unavailable before navigation: {links:?}"
        );
    }
}
