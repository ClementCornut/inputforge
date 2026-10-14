//! Real renderer batch publication follows the existing app poller.
use super::*;
use dioxus::core::{AttributeValue, DynamicNode, ElementId, TemplateNode};
#[test]
#[expect(
    clippy::await_holding_lock,
    reason = "hold writer across tick to prove nonblocking skip"
)]
fn batch_capability_reconciliation_uses_existing_tick_without_duplicate_polling() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let (state, id) = fixture();
            let mut s = state.write();
            let mut info = s.device_registry[&id].info.clone();
            info.id = DeviceId("batch".into());
            info.buttons = 4;
            s.devices.push(inputforge_core::state::DeviceState {
                info,
                connected: true,
                diagnostics: DeviceDiagnostics::default(),
            });
            drop(s);
            let starts = Rc::new(Cell::new(0));
            let mut dom = VirtualDom::new_with_props(
                editor,
                EditorProps {
                    state: Arc::clone(&state),
                    starts: Rc::clone(&starts),
                },
            );
            dom.rebuild_in_place();
            drive(&mut dom, 0).await;
            click(&dom, "Batch map");
            dom.render_immediate_to_vec();
            drive(&mut dom, 0).await;
            assert!(dioxus_ssr::render(&dom).contains("Apply 4 mappings"));
            state.write().virtual_devices[0].button_count = 2;
            drive(&mut dom, 15).await;
            assert!(dioxus_ssr::render(&dom).contains("Apply 4 mappings"));
            let writer = state.write();
            drive(&mut dom, 1).await;
            assert!(dioxus_ssr::render(&dom).contains("Apply 4 mappings"));
            drop(writer);
            drive(&mut dom, 16).await;
            assert!(dioxus_ssr::render(&dom).contains("Apply 2 mappings"));
            assert_eq!(starts.get(), 1);
            drop(dom);
            tokio::time::advance(Duration::from_millis(32)).await;
            assert_eq!(Arc::strong_count(&state), 1);
        });
}
fn text(node: &VNode, current: &TemplateNode, label: &str, dom: &VirtualDom) -> bool {
    match current {
        TemplateNode::Text { text: value } => *value == label,
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
                    let child = scope.root_node();
                    child
                        .template
                        .roots
                        .iter()
                        .any(|root| text(child, root, label, dom))
                }),
            DynamicNode::Placeholder(_) => false,
        },
    }
}
pub(super) fn element(dom: &VirtualDom, node: &VNode, label: &str) -> Option<ElementId> {
    element_in(dom, node, label, false)
}
fn element_in(dom: &VirtualDom, node: &VNode, label: &str, row: bool) -> Option<ElementId> {
    for (index, path) in node.template.attr_paths.iter().enumerate() {
        let mut current = &node.template.roots[usize::from(path[0])];
        for offset in &path[1..] {
            if let TemplateNode::Element { children, .. } = current {
                current = &children[usize::from(*offset)];
            }
        }
        if (matches!(current, TemplateNode::Element { tag: "button", .. }) && !row
            || row
                && matches!(current, TemplateNode::Element { tag: "div", attrs, .. } if attrs.iter().any(|attr| matches!(attr, dioxus::core::TemplateAttribute::Static { name: "role", value: "button", .. }))))
            && (node.dynamic_attrs[index].iter().any(|a| {
                a.name == "aria-label"
                    && matches!(&a.value, AttributeValue::Text(value) if value == label)
            }) || text(node, current, label, dom))
        {
            return node.mounted_dynamic_attribute(index, dom);
        }
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(i, child)| match child {
            DynamicNode::Component(c) => c
                .mounted_scope(i, node, dom)
                .and_then(|s| element_in(dom, s.root_node(), label, row)),
            DynamicNode::Fragment(nodes) => {
                nodes.iter().find_map(|n| element_in(dom, n, label, row))
            }
            _ => None,
        })
}
pub(super) fn click(dom: &VirtualDom, label: &str) {
    let id = element(dom, dom.base_scope().root_node(), label)
        .unwrap_or_else(|| panic!("missing batch control {label}"));
    dom.runtime().handle_event(
        "click",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(
                SerializedMouseData::default(),
            ))) as Rc<dyn std::any::Any>,
            false,
        ),
        id,
    );
}
