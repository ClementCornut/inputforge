//! Focus returns only after the app accepts a controlled menu's dismissal.
use super::{controls, document};
use dioxus::prelude::*;
use inputforge_gui_dx::components::{AnchoredMenu, MenuAnchor, MenuItem};
use std::{cell::Cell, rc::Rc};

fn menu() -> Element {
    inputforge_app::ui::use_provider();
    let accept = use_context::<Rc<Cell<bool>>>();
    let mut anchor = use_signal(|| Some(MenuAnchor { x: 20.0, y: 20.0 }));
    rsx! {
        AnchoredMenu {
            id: "acknowledged-cursor-menu",
            open: anchor(),
            return_focus: "cursor-menu-opener",
            on_close: move |_| {
                if accept.get() { anchor.set(None); }
                accept.get()
            },
            MenuItem { onclick: move |_| {}, "Preview action" }
        }
    }
}

fn exercise(accept: bool, activate: bool) -> (String, Vec<String>) {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let doc = Rc::new(document::Document::default());
    let mut dom = VirtualDom::new(menu);
    dom.provide_root_context(Rc::new(Cell::new(accept)));
    dom.provide_root_context(Rc::clone(&doc) as Rc<dyn dioxus::document::Document>);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    let before = doc.scripts().len();
    if activate {
        controls::event(&dom, "click", 0, SerializedMouseData::default());
    } else {
        controls::key(&dom, 0, Key::Escape);
    }
    dom.render_immediate_to_vec();
    let scripts = doc.scripts()[before..].to_vec();
    (dioxus_ssr::render(&dom), scripts)
}

fn assert_rejection_keeps_menu_and_focus(activate: bool) {
    let (html, scripts) = exercise(false, activate);
    assert!(html.contains("id=\"acknowledged-cursor-menu\""));
    assert!(
        !scripts
            .iter()
            .any(|script| script.contains("cursor-menu-opener") && script.contains("?.focus();")),
        "rejected closure moved focus outside the retained menu: {scripts:?}"
    );
}

#[test]
fn adoption_cursor_closure_rejection_keeps_focus_after_activation() {
    assert_rejection_keeps_menu_and_focus(true);
}

#[test]
fn adoption_cursor_closure_rejection_keeps_focus_after_escape() {
    assert_rejection_keeps_menu_and_focus(false);
}

#[test]
fn adoption_cursor_closure_acceptance_restores_focus_for_activation_and_escape() {
    for activate in [true, false] {
        let (html, scripts) = exercise(true, activate);
        assert!(!html.contains("id=\"acknowledged-cursor-menu\""));
        assert!(
            scripts.iter().any(
                |script| script.contains("cursor-menu-opener") && script.contains("?.focus();")
            ),
            "accepted closure did not restore opener focus: {scripts:?}"
        );
    }
}
