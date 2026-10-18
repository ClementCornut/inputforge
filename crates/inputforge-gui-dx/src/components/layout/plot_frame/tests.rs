//! Rendered plot naming and event forwarding contracts.

use std::{cell::RefCell, rc::Rc};

use dioxus::{core::Mutation, html::SerializedHtmlEventConverter, prelude::*};

use super::PlotFrame;

#[test]
fn descriptions_are_local_to_each_named_plot() {
    fn fixture() -> Element {
        rsx! {
            PlotFrame { id: "curve-1", label: "Response curve", description: "Input against output",
                instructions: "Use arrows", value_text: "Point one: 0.5", svg {} }
            PlotFrame { id: "zone-1", label: "Deadzone", instructions: "Move handles", svg {} }
        }
    }
    let mut dom = VirtualDom::new(fixture);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains(
            "aria-describedby=\"curve-1-description curve-1-instructions curve-1-value\""
        )
    );
    assert!(html.contains("aria-describedby=\"zone-1-instructions\""));
    assert_eq!(html.matches("role=\"group\"").count(), 2);
    assert_eq!(html.matches("tabindex=\"0\"").count(), 2);
    for text in [
        "Response curve",
        "Input against output",
        "Use arrows",
        "Point one: 0.5",
    ] {
        assert!(html.contains(text));
    }
    assert!(!html.contains("aria-live="));
}

#[test]
fn absent_descriptions_do_not_create_empty_references() {
    fn fixture() -> Element {
        rsx! { PlotFrame { id: "plot", label: "Curve", svg {} } }
    }
    let mut dom = VirtualDom::new(fixture);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(!html.contains("aria-describedby="));
    assert!(!html.contains("if-sr-only"));
}

#[test]
fn plot_events_reach_the_same_app_callbacks() {
    fn fixture() -> Element {
        let events = use_context::<Rc<RefCell<Vec<&'static str>>>>();
        let mount = Rc::clone(&events);
        let key = Rc::clone(&events);
        let blur = Rc::clone(&events);
        let down = Rc::clone(&events);
        let movement = Rc::clone(&events);
        let up = Rc::clone(&events);
        rsx! { PlotFrame { id: "plot", label: "Curve",
            onmounted: move |_| mount.borrow_mut().push("mounted"),
            onkeydown: move |_| key.borrow_mut().push("key"),
            onfocusout: move |_| blur.borrow_mut().push("blur"),
            onpointerdown: move |_| down.borrow_mut().push("down"),
            onpointermove: move |_| movement.borrow_mut().push("move"),
            onpointerup: move |_| up.borrow_mut().push("up"),
            onpointercancel: move |_| events.borrow_mut().push("cancel"),
            svg {}
        } }
    }
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let events = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let mut dom = VirtualDom::new(fixture);
    dom.provide_root_context(Rc::clone(&events));
    let mounted = dom.rebuild_to_vec();
    for (name, data) in [
        ("mounted", PlatformEventData::new(Box::new(()))),
        (
            "keydown",
            PlatformEventData::new(Box::new(SerializedKeyboardData::new(
                Key::ArrowRight,
                Code::ArrowRight,
                Location::Standard,
                false,
                Modifiers::empty(),
                false,
            ))),
        ),
        (
            "focusout",
            PlatformEventData::new(Box::new(SerializedFocusData::default())),
        ),
        ("pointerdown", PlatformEventData::new(Box::new(pointer()))),
        ("pointermove", PlatformEventData::new(Box::new(pointer()))),
        ("pointerup", PlatformEventData::new(Box::new(pointer()))),
        ("pointercancel", PlatformEventData::new(Box::new(pointer()))),
    ] {
        let element = mounted
            .edits
            .iter()
            .find_map(|mutation| match mutation {
                Mutation::NewEventListener { name: listener, id } if *listener == name => Some(*id),
                _ => None,
            })
            .expect("listener on rendered plot");
        dom.runtime().handle_event(
            name,
            Event::new(Rc::new(data) as Rc<dyn std::any::Any>, false),
            element,
        );
    }
    assert_eq!(
        *events.borrow(),
        vec!["mounted", "key", "blur", "down", "move", "up", "cancel"]
    );
}

fn pointer() -> SerializedPointerData {
    serde_json::from_value(serde_json::json!({
        "alt_key": false, "button": 0, "buttons": 1,
        "client_x": 10.0, "client_y": 20.0,
        "ctrl_key": false, "meta_key": false, "shift_key": false,
        "offset_x": 10.0, "offset_y": 20.0,
        "page_x": 10.0, "page_y": 20.0, "screen_x": 10.0, "screen_y": 20.0,
        "pointer_id": 1, "width": 1.0, "height": 1.0, "pressure": 0.5,
        "tangential_pressure": 0.0, "tilt_x": 0, "tilt_y": 0, "twist": 0,
        "pointer_type": "mouse", "is_primary": true
    }))
    .expect("valid serialized pointer event")
}
