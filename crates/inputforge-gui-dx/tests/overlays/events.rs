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
pub(super) fn event<T: 'static>(
    dom: &VirtualDom,
    name: &str,
    index: usize,
    data: T,
) -> Event<dyn std::any::Any> {
    let mut ids = vec![];
    listeners(dom, dom.base_scope().root_node(), name, &mut ids);
    let evt = Event::new(
        Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event(name, evt.clone(), ids[index]);
    evt
}
pub(super) fn key(dom: &VirtualDom, index: usize, key: Key) -> Event<dyn std::any::Any> {
    event(
        dom,
        "keydown",
        index,
        SerializedKeyboardData::new(
            key,
            Code::Unidentified,
            Location::Standard,
            false,
            Modifiers::empty(),
            false,
        ),
    )
}
