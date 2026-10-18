//! Real overlay handlers use the same app behavior as the editor.
#[path = "overlays/acknowledged_menu.rs"]
mod acknowledged_menu;
#[path = "overlays/events.rs"]
mod controls;
#[path = "overlays/dialog.rs"]
mod dialog_tests;
#[path = "overlays/dirty.rs"]
mod dirty;
#[path = "overlays/document.rs"]
mod document;
#[path = "overlays/escape.rs"]
mod escape;
#[path = "overlays/lifecycle.rs"]
mod lifecycle;
#[path = "overlays/shortcuts.rs"]
mod shortcuts;
use controls::{event, key};
use dioxus::prelude::*;
use inputforge_gui_dx::{
    components::{
        ClickAwayListener, DialogBody, DialogDescription, DialogRoot, DialogTitle, MenuItem,
        MenuItems, MenuRoot, MenuTrigger,
    },
    patterns::DestructiveConfirmDialog,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
struct State {
    saw_open: Cell<bool>,
    accept: Cell<bool>,
    open: RefCell<Option<Signal<bool>>>,
}
fn root() -> Element {
    inputforge_app::ui::use_provider();
    let open = use_signal(|| true);
    let state = use_context::<Rc<State>>();
    *state.open.borrow_mut() = Some(open);
    rsx! { DestructiveConfirmDialog { open, description: rsx! {"Delete?"}, confirm_label: "Delete".to_owned(), oncancel: move |()| true, onconfirm: move |()| { state.saw_open.set(open()); state.accept.get() } } }
}
#[test]
fn confirmation_runs_before_any_visibility_change() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let state = Rc::new(State::default());
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&state));
    dom.rebuild_in_place();
    event(&dom, "click", 3, SerializedMouseData::default());
    assert!(
        state.saw_open.get(),
        "the action must run while its confirmation is still open"
    );
    assert!(state.open.borrow().unwrap()());
    state.accept.set(true);
    event(&dom, "click", 3, SerializedMouseData::default());
    assert!(!state.open.borrow().unwrap()());
}
fn menu() -> Element {
    inputforge_app::ui::use_provider();
    rsx! {MenuRoot {MenuTrigger {"Open"} MenuItems {MenuItem {"One"} MenuItem {disabled: true,"Disabled"}}}}
}
#[test]
fn menu_arrows_prevent_scrolling_and_escape_does_not_bubble() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(menu);
    dom.rebuild_in_place();
    event(&dom, "click", 0, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    let arrow = key(&dom, 0, Key::ArrowDown);
    assert!(
        !arrow.default_action_enabled(),
        "menu traversal must not also scroll the page"
    );
    assert!(!arrow.propagates());
    let escape = key(&dom, 0, Key::Escape);
    assert!(!escape.propagates());
}
fn rejecting_menu() -> Element {
    inputforge_app::ui::use_provider();
    let state = use_context::<Rc<State>>();
    rsx! {MenuRoot {MenuTrigger {"Open"} MenuItems {
        MenuItem {onactivate: move |_| { state.saw_open.set(true); false }, "Reject"}
        MenuItem {disabled: true, "Disabled"}
    }}}
}
#[test]
fn closed_or_disabled_menu_items_cannot_activate_and_failed_activation_stays_open() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let state = Rc::new(State::default());
    let mut dom = VirtualDom::new(rejecting_menu);
    dom.provide_root_context(Rc::clone(&state));
    dom.rebuild_in_place();
    event(&dom, "click", 1, SerializedMouseData::default());
    assert!(
        !state.saw_open.get(),
        "closed menus must ignore stale activation"
    );
    event(&dom, "click", 0, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    event(&dom, "click", 2, SerializedMouseData::default());
    assert!(!state.saw_open.get());
    event(&dom, "click", 1, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    assert!(state.saw_open.get());
    assert!(dioxus_ssr::render(&dom).contains("aria-expanded=\"true\""));
}
#[test]
fn menu_native_activation_tab_and_keyboard_focus_return_remain_distinct() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let doc = Rc::new(document::Document::default());
    let mut dom = VirtualDom::new(menu);
    dom.provide_root_context(Rc::clone(&doc) as Rc<dyn dioxus::document::Document>);
    dom.rebuild_in_place();
    event(&dom, "click", 0, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    let id = html
        .split("aria-controls=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(html.contains(&format!("id=\"{id}\"")));
    for k in [Key::ArrowDown, Key::ArrowUp, Key::Home, Key::End] {
        let e = key(&dom, 0, k);
        assert!(!e.default_action_enabled());
        assert!(!e.propagates());
    }
    for k in [Key::Enter, Key::Character(" ".into())] {
        assert!(key(&dom, 0, k).default_action_enabled());
    }
    let scripts = doc.scripts().len();
    let tab = key(&dom, 0, Key::Tab);
    assert!(tab.default_action_enabled());
    assert_eq!(doc.scripts().len(), scripts, "Tab must not restore focus");
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("aria-expanded=\"false\""));
    event(&dom, "click", 0, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    key(&dom, 0, Key::Escape);
    assert!(
        doc.scripts()
            .iter()
            .any(|s| s.contains("-trigger\")?.focus();")),
        "Escape must request the current trigger, safely if still mounted"
    );
    dom.render_immediate_to_vec();
    event(&dom, "click", 0, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    event(&dom, "click", 1, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("aria-expanded=\"false\""));
}
