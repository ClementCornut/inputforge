use dioxus::{
    core::{AttributeValue, DynamicNode, ElementId, TemplateAttribute, TemplateNode},
    prelude::*,
};
use std::rc::Rc;
fn text(node: &VNode, current: &TemplateNode, label: &str, dom: &VirtualDom) -> bool {
    match current {
        TemplateNode::Text { text } => *text == label,
        TemplateNode::Element { children, .. } => {
            children.iter().any(|child| text(node, child, label, dom))
        }
        TemplateNode::Dynamic { id } => match &node.dynamic_nodes[*id] {
            DynamicNode::Text(value) => value.value == label,
            DynamicNode::Fragment(nodes) => nodes.iter().any(|child| {
                child
                    .template
                    .roots
                    .iter()
                    .any(|root| text(child, root, label, dom))
            }),
            DynamicNode::Component(component) => component
                .mounted_scope(*id, node, dom)
                .is_some_and(|scope| {
                    scope
                        .root_node()
                        .template
                        .roots
                        .iter()
                        .any(|root| text(scope.root_node(), root, label, dom))
                }),
            DynamicNode::Placeholder(_) => false,
        },
    }
}
fn element(dom: &VirtualDom, node: &VNode, label: &str) -> Option<ElementId> {
    for (index, path) in node.template.attr_paths.iter().enumerate() {
        let mut current = &node.template.roots[usize::from(path[0])];
        for offset in &path[1..] {
            if let TemplateNode::Element { children, .. } = current {
                current = &children[usize::from(*offset)];
            }
        }
        let static_label = matches!(current,TemplateNode::Element{attrs,..} if attrs.iter().any(|a|matches!(a,TemplateAttribute::Static{name,value,..} if (*name=="aria-label" || *name=="id" || *name=="class") && *value==label)));
        if static_label || node.dynamic_attrs[index].iter().any(|a|matches!(&a.value,AttributeValue::Text(value) if (a.name=="aria-label" || a.name=="id") && value==label)) || (matches!(current,TemplateNode::Element{tag:"button",..}) && text(node,current,label,dom)) {return node.mounted_dynamic_attribute(index,dom);}
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(i, child)| match child {
            DynamicNode::Component(c) => c
                .mounted_scope(i, node, dom)
                .and_then(|s| element(dom, s.root_node(), label)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|n| element(dom, n, label)),
            _ => None,
        })
}
pub(super) fn send(dom: &VirtualDom, label: &str, event: &str, data: PlatformEventData) {
    let id = element(dom, dom.base_scope().root_node(), label)
        .unwrap_or_else(|| panic!("missing {label}"));
    dom.runtime().handle_event(
        event,
        Event::new(Rc::new(data) as Rc<dyn std::any::Any>, false),
        id,
    );
}
pub(super) fn key(dom: &VirtualDom, label: &str, key: Key, code: Code) {
    send(
        dom,
        label,
        "keydown",
        PlatformEventData::new(Box::new(SerializedKeyboardData::new(
            key,
            code,
            Location::Standard,
            false,
            Modifiers::empty(),
            false,
        ))),
    );
}
pub(super) fn click(dom: &VirtualDom, label: &str) {
    send(
        dom,
        label,
        "click",
        PlatformEventData::new(Box::new(SerializedMouseData::default())),
    );
}
