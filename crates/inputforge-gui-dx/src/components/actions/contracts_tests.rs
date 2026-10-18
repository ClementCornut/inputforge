use super::{button::Button, icon_button::IconButton};
use crate::components::test_events as events;
use crate::icons::Icon as IconKind;
use dioxus::prelude::*;

#[test]
fn action_controls_do_not_submit_enclosing_forms() {
    fn fixture() -> Element {
        rsx! { form { Button { "Action" } IconButton { icon: IconKind::Plus, label: "Add" } } }
    }
    let mut dom = VirtualDom::new(fixture);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert_eq!(html.matches("type=\"button\"").count(), 2, "{html}");
}

#[test]
fn action_button_ignores_stale_disabled_activation() {
    use std::{cell::Cell, rc::Rc};
    fn fixture() -> Element {
        let calls = use_context::<Rc<Cell<usize>>>();
        rsx! { Button { disabled: true, onclick: move |_| calls.set(calls.get() + 1), "Disabled" } }
    }
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let calls = Rc::new(Cell::new(0_usize));
    let mut dom = VirtualDom::new(fixture);
    dom.provide_root_context(Rc::clone(&calls));
    dom.rebuild_in_place();
    events::event(&dom, "click", 0, SerializedMouseData::default());
    assert_eq!(calls.get(), 0);
}

#[test]
fn action_buttons_leave_enter_and_space_to_native_activation() {
    fn fixture() -> Element {
        rsx! { Button { "Action" } }
    }
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(fixture);
    dom.rebuild_in_place();
    for key in [Key::Enter, Key::Character(" ".into())] {
        let event = events::key(&dom, 0, key);
        assert!(event.default_action_enabled());
    }
}

#[test]
fn contextual_icon_name_and_disclosure_metadata_reach_native_controls() {
    fn fixture() -> Element {
        let profile = "Flight";
        rsx! {
            Button {
                id: "snapshots", title: "Show stored states", aria_label: "Snapshots",
                aria_labelledby: "snapshot-heading", aria_describedby: "snapshot-help",
                aria_pressed: true, aria_expanded: false, aria_controls: "snapshot-body", aria_haspopup: "menu",
                "Snapshots"
            }
            IconButton { icon: IconKind::Plus, label: format!("Add to {profile}"), title: "Add mapping" }
        }
    }
    let mut dom = VirtualDom::new(fixture);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    for attr in [
        "id=\"snapshots\"",
        "aria-labelledby=\"snapshot-heading\"",
        "aria-describedby=\"snapshot-help\"",
        "aria-pressed=\"true\"",
        "aria-expanded=\"false\"",
        "aria-controls=\"snapshot-body\"",
        "aria-haspopup=\"menu\"",
        "aria-label=\"Add to Flight\"",
    ] {
        assert!(html.contains(attr), "missing {attr}: {html}");
    }
}

#[test]
fn compound_rows_allow_editors_without_nesting_native_buttons() {
    use super::surfaces::{SelectableRow, ToggleChip};
    fn fixture() -> Element {
        rsx! {
            SelectableRow { selected: true, data_sortable_row: true, aria_label: "Mapping",
                input { "aria-label": "Mapping name" }
                Button { "Drag handle" }
            }
            ToggleChip { pressed: true, disabled: true, onclick: move |_| {}, "Replace" }
        }
    }
    let mut dom = VirtualDom::new(fixture);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    let doc = scraper::Html::parse_fragment(&html);
    let selector = scraper::Selector::parse("button button, button input").unwrap();
    assert_eq!(doc.select(&selector).count(), 0, "{html}");
    assert!(html.contains("role=\"group\""), "{html}");
    assert!(html.contains("data-sortable-row=\"true\""), "{html}");
    let chips =
        scraper::Selector::parse("button.if-toggle-chip[aria-pressed='true'][disabled]").unwrap();
    assert_eq!(doc.select(&chips).count(), 1, "{html}");
}
