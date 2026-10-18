//! Native observations retain exact raw payloads and browser-owned drafts.
use super::{native_number::NativeNumberInput, text_input::TextInput};
use dioxus::prelude::*;
use std::rc::Rc;

fn listeners(dom: &VirtualDom, node: &VNode, name: &str, ids: &mut Vec<dioxus::core::ElementId>) {
    use dioxus::core::DynamicNode;
    for (i, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs
            .iter()
            .any(|a| a.name == name || a.name == format!("on{name}"))
            && let Some(id) = node.mounted_dynamic_attribute(i, dom)
        {
            ids.push(id);
        }
    }
    for (i, child) in node.dynamic_nodes.iter().enumerate() {
        match child {
            DynamicNode::Component(c) => {
                if let Some(s) = c.mounted_scope(i, node, dom) {
                    listeners(dom, s.root_node(), name, ids);
                }
            }
            DynamicNode::Fragment(nodes) => {
                for n in nodes {
                    listeners(dom, n, name, ids);
                }
            }
            _ => {}
        }
    }
}

fn event(dom: &VirtualDom, name: &str, raw: &str) {
    let mut ids = vec![];
    listeners(dom, dom.base_scope().root_node(), name, &mut ids);
    dom.runtime().handle_event(
        name,
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                raw.into(),
                vec![],
            )))) as Rc<dyn std::any::Any>,
            false,
        ),
        ids[0],
    );
}

#[test]
fn native_number_forwards_raw_change_without_parse_or_clamp() {
    fn root() -> Element {
        let mut observed = use_signal(String::new);
        rsx! {
            NativeNumberInput { value: "4", min: "0", max: "8", step: "1", onchange: move |event: FormEvent| observed.set(event.value()) }
            output { "{observed}" }
        }
    }
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    for raw in ["", "-1", "99999999999999999999999", "not-a-number", "4.5"] {
        event(&dom, "change", raw);
        dom.render_immediate_to_vec();
        let rendered = dioxus_ssr::render(&dom);
        assert!(
            rendered.contains(&format!("<output>{raw}</output>")),
            "{rendered}"
        );
    }
}

#[test]
fn text_rapid_unicode_typing_keeps_initial_value_transport() {
    fn root() -> Element {
        let mut value = use_signal(String::new);
        rsx! { TextInput { value, oninput: move |event: FormEvent| value.set(event.value()) } }
    }
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    for raw in ["D", "Dev", "Device été 🕹"] {
        event(&dom, "input", raw);
    }
    let mutations = dom.render_immediate_to_vec();
    assert!(
        !mutations.edits.iter().any(|edit| matches!(
            edit,
            dioxus::core::Mutation::SetAttribute { name: "value", .. }
        )),
        "{:?}",
        mutations.edits
    );
    let rendered = dioxus_ssr::render(&dom);
    assert!(rendered.contains("Device été 🕹"), "{rendered}");
    assert!(rendered.contains("data-if-text-owner="), "{rendered}");
}
