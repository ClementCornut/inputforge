//! Notification primitives preserve app-owned interaction observations.
use dioxus::prelude::*;
use inputforge_gui_dx::toast::{Interaction, Toast, ToastLevel, ToastQueue, ToastViewport};
use std::{cell::RefCell, rc::Rc};

fn harness() -> Element {
    let events = use_context::<Rc<RefCell<Vec<(u64, Interaction)>>>>();
    let visible = use_signal(|| {
        vec![
            Toast {
                id: 7,
                level: ToastLevel::Warning,
                message: "Repeated warning".into(),
                count: 3,
            },
            Toast {
                id: 8,
                level: ToastLevel::Success,
                message: "Saved".into(),
                count: 1,
            },
        ]
    });
    use_context_provider(|| ToastQueue {
        visible,
        push: Callback::new(|_| {}),
        event: Callback::new(move |event| events.borrow_mut().push(event)),
        tick: Callback::new(|_| {}),
        dispose: Callback::new(|()| {}),
    });
    rsx! { ToastViewport {} }
}

#[test]
fn adoption_toast_dismiss_count_roles_and_focus_hosts() {
    let mut dom = VirtualDom::new(harness);
    dom.provide_root_context(Rc::new(RefCell::new(Vec::<(u64, Interaction)>::new())));
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    let document = scraper::Html::parse_fragment(&html);
    for (selector, expected) in [
        (".if-toast-stack[role=status][aria-live=polite]", 1),
        (".if-toast[tabindex='0']", 2),
        (".if-toast[role=alert]", 1),
        ("button.if-icon-button[aria-label=Dismiss][type=button]", 2),
        (".if-badge--warning.if-badge--count", 1),
    ] {
        assert_eq!(
            document
                .select(&scraper::Selector::parse(selector).unwrap())
                .count(),
            expected,
            "{selector}: {html}"
        );
    }
    let count = document
        .select(&scraper::Selector::parse(".if-badge--count").unwrap())
        .next()
        .unwrap();
    assert_eq!(count.text().collect::<String>(), "×3");
    assert!(count.value().attr("role").is_none());
    assert!(count.value().attr("aria-live").is_none());
}

fn listeners(dom: &VirtualDom, node: &VNode, name: &str, ids: &mut Vec<dioxus::core::ElementId>) {
    use dioxus::core::DynamicNode;
    for (i, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs
            .iter()
            .any(|attribute| attribute.name == name || attribute.name == format!("on{name}"))
            && let Some(id) = node.mounted_dynamic_attribute(i, dom)
        {
            ids.push(id);
        }
    }
    for (i, child) in node.dynamic_nodes.iter().enumerate() {
        match child {
            DynamicNode::Component(component) => {
                if let Some(scope) = component.mounted_scope(i, node, dom) {
                    listeners(dom, scope.root_node(), name, ids);
                }
            }
            DynamicNode::Fragment(nodes) => {
                for child in nodes {
                    listeners(dom, child, name, ids);
                }
            }
            _ => {}
        }
    }
}

fn event<T: 'static>(dom: &VirtualDom, name: &str, data: T) {
    let mut ids = vec![];
    listeners(dom, dom.base_scope().root_node(), name, &mut ids);
    dom.runtime().handle_event(
        name,
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
            false,
        ),
        ids[0],
    );
}

#[test]
fn adoption_toast_forwards_pause_escape_and_dismiss_to_app() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let events = Rc::new(RefCell::new(Vec::<(u64, Interaction)>::new()));
    let mut dom = VirtualDom::new(harness);
    dom.provide_root_context(Rc::clone(&events));
    dom.rebuild_in_place();
    for name in ["mouseenter", "mouseleave"] {
        event(&dom, name, SerializedMouseData::default());
    }
    for name in ["focusin", "focusout"] {
        event(&dom, name, SerializedFocusData::default());
    }
    event(
        &dom,
        "keydown",
        SerializedKeyboardData::new(
            Key::Escape,
            Code::Escape,
            Location::Standard,
            false,
            Modifiers::empty(),
            false,
        ),
    );
    event(&dom, "click", SerializedMouseData::default());
    assert_eq!(
        *events.borrow(),
        [
            (7, Interaction::Hover(true)),
            (7, Interaction::Hover(false)),
            (7, Interaction::Focus(true)),
            (7, Interaction::Focus(false)),
            (7, Interaction::Key(Key::Escape)),
            (7, Interaction::Dismiss),
        ]
    );
}
