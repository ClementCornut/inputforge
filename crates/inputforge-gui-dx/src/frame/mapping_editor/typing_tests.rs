//! Native input mutations must not echo old mapping-name drafts.
use super::{HarnessComponent, HarnessProps, seeded_profile_with_one_mapping};
use dioxus::prelude::*;
use inputforge_core::{
    action::Action,
    types::{DeviceId, InputAddress, InputId},
};
use parking_lot::RwLock;
use std::{rc::Rc, sync::Arc};

fn root(props: HarnessProps) -> Element {
    let element = HarnessComponent(props);
    crate::patterns::keyboard_capture::use_keyboard_capture_provider();
    element
}

fn listener(dom: &VirtualDom, node: &VNode, name: &str) -> Option<dioxus::core::ElementId> {
    use dioxus::core::DynamicNode;
    for (index, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs
            .iter()
            .any(|attr| attr.name == name || attr.name == format!("on{name}"))
        {
            return node.mounted_dynamic_attribute(index, dom);
        }
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(index, child)| match child {
            DynamicNode::Component(component) => component
                .mounted_scope(index, node, dom)
                .and_then(|scope| listener(dom, scope.root_node(), name)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|node| listener(dom, node, name)),
            _ => None,
        })
}
fn event<T: 'static>(dom: &VirtualDom, name: &str, data: T) {
    let id = listener(dom, dom.base_scope().root_node(), name).unwrap();
    dom.runtime().handle_event(
        name,
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
            false,
        ),
        id,
    );
}
fn key(dom: &VirtualDom, key: Key) {
    event(
        dom,
        "keydown",
        SerializedKeyboardData::new(
            key,
            Code::Unidentified,
            Location::Standard,
            false,
            Modifiers::empty(),
            false,
        ),
    );
}
#[test]
fn rapid_header_rename_does_not_echo_values_and_retains_failed_exact_draft() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let addr = InputAddress::Bound {
        device: DeviceId("dev-1".into()),
        input: InputId::Axis { index: 0 },
    };
    let mut dom = VirtualDom::new_with_props(
        root,
        HarnessProps {
            state: Arc::new(RwLock::new(seeded_profile_with_one_mapping(vec![
                Action::Invert,
            ]))),
            addr,
            current_mode: None,
            initial_live: None,
            engine_status: None,
        },
    );
    dom.rebuild_in_place();
    event(&dom, "contextmenu", SerializedMouseData::default());
    dom.render_immediate_to_vec();
    for raw in ["M", "Map", "Mapping été 🕹"] {
        event(&dom, "input", SerializedFormData::new(raw.into(), vec![]));
    }
    let mutations = dom.render_immediate_to_vec();
    assert!(
        !mutations.edits.iter().any(|edit| matches!(
            edit,
            dioxus::core::Mutation::SetAttribute { name: "value", .. }
        )),
        "header typing echoed an old browser value: {:?}",
        mutations.edits
    );
    key(&dom, Key::Enter);
    event(&dom, "blur", SerializedFocusData {});
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("Mapping été 🕹"),
        "failed rename lost exact text: {html}"
    );
    assert!(
        html.contains("if-editor__title-input"),
        "failed rename closed: {html}"
    );
    key(&dom, Key::Escape);
    dom.render_immediate_to_vec();
    assert!(!dioxus_ssr::render(&dom).contains("if-editor__title-input"));
}
