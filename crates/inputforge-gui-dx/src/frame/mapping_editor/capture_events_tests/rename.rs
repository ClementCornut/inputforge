//! Retained header text keeps its original mapping across published prop updates.
use super::{Surface, fixture, primary};
use dioxus::{
    core::{DynamicNode, ElementId},
    html::SerializedHtmlEventConverter,
    prelude::*,
};
use inputforge_core::action::Action;
use std::{rc::Rc, sync::mpsc};
fn listener(dom: &VirtualDom, node: &VNode, name: &str) -> Option<ElementId> {
    for (i, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs
            .iter()
            .any(|a| a.name == name || a.name == format!("on{name}"))
        {
            return node.mounted_dynamic_attribute(i, dom);
        }
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(i, child)| match child {
            DynamicNode::Component(c) => c
                .mounted_scope(i, node, dom)
                .and_then(|s| listener(dom, s.root_node(), name)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|n| listener(dom, n, name)),
            _ => None,
        })
}
fn send(dom: &VirtualDom, name: &str, data: PlatformEventData) {
    let id = listener(dom, dom.base_scope().root_node(), name).unwrap();
    dom.runtime().handle_event(
        name,
        Event::new(Rc::new(data) as Rc<dyn std::any::Any>, false),
        id,
    );
}
#[test]
fn retained_header_draft_rejects_same_key_replacement_after_props_rerender() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (mut dom, f, rx) = fixture(Surface::Header);
    send(
        &dom,
        "keydown",
        PlatformEventData::new(Box::new(SerializedKeyboardData::new(
            Key::F2,
            Code::F2,
            Location::Standard,
            false,
            Modifiers::empty(),
            false,
        ))),
    );
    dom.render_immediate_to_vec();
    send(
        &dom,
        "input",
        PlatformEventData::new(Box::new(SerializedFormData::new(
            "Original header draft".into(),
            vec![],
        ))),
    );
    dom.render_immediate_to_vec();
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(
            &primary(),
            "Default",
            Some("Fixture".into()),
            vec![Action::Invert],
        );
    let key = ("Default".into(), primary());
    f.config
        .borrow()
        .unwrap()
        .set(inputforge_app::bridge::prepare(&f.state.read(), Some(&key)).config);
    dom.render_immediate_to_vec();
    send(
        &dom,
        "blur",
        PlatformEventData::new(Box::new(SerializedFocusData {})),
    );
    dom.render_immediate_to_vec();
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Original header draft"), "{html}");
    let editor = f.editor.borrow().unwrap();
    assert!(editor.error.peek().is_some());
    assert!(editor.undo_log.peek().stacks.is_empty());
}
