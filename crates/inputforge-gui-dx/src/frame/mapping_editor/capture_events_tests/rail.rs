//! Actual Add button and Enter handlers preserve delivery and navigation ordering.
use super::{Surface, address, fixture};
use dioxus::{
    core::{AttributeValue, DynamicNode, ElementId, TemplateNode},
    html::SerializedHtmlEventConverter,
    prelude::*,
};
use inputforge_core::{
    engine::EngineCommand,
    types::{InputId, InputValue},
};
use std::{
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};
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
fn element(dom: &VirtualDom, node: &VNode, label: &str) -> Option<ElementId> {
    for (index, path) in node.template.attr_paths.iter().enumerate() {
        let mut current = &node.template.roots[usize::from(path[0])];
        for offset in &path[1..] {
            if let TemplateNode::Element { children, .. } = current {
                current = &children[usize::from(*offset)];
            }
        }
        if matches!(current, TemplateNode::Element { tag: "button", .. })
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
                .and_then(|s| element(dom, s.root_node(), label)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|n| element(dom, n, label)),
            _ => None,
        })
}
fn click(dom: &VirtualDom, label: &str) {
    let id = element(dom, dom.base_scope().root_node(), label)
        .unwrap_or_else(|| panic!("missing rail control {label}"));
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

fn captured() -> (
    VirtualDom,
    Rc<fixture::Fixture>,
    mpsc::Receiver<EngineCommand>,
) {
    captured_input(3)
}
fn captured_input(
    index: u8,
) -> (
    VirtualDom,
    Rc<fixture::Fixture>,
    mpsc::Receiver<EngineCommand>,
) {
    let (mut dom, f, rx) = fixture(Surface::Add);
    super::click(&dom, "Add mapping");
    dom.render_immediate_to_vec();
    dom.in_scope(ScopeId::APP, || {
        let c = f.controller.borrow().as_ref().unwrap().clone();
        let now = Instant::now();
        f.state.write().input_cache.update(
            &address(InputId::Button { index }),
            &InputValue::Button { pressed: false },
        );
        c.sample(&f.state.read(), now);
        f.state.write().input_cache.update(
            &address(InputId::Button { index }),
            &InputValue::Button { pressed: true },
        );
        c.sample(&f.state.read(), now + Duration::from_millis(16));
        c.sample(&f.state.read(), now + Duration::from_millis(80));
        c.deliver();
    });
    dom.render_immediate_to_vec();
    (dom, f, rx)
}
#[test]
fn actual_add_button_dispatches_before_history_and_navigation() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (mut dom, f, rx) = captured();
    click(&dom, "Add");
    dom.render_immediate_to_vec();
    assert!(
        matches!(rx.try_recv().unwrap(), EngineCommand::SetMapping { input, name: None, actions, .. } if input == address(InputId::Button { index: 3 }) && actions.is_empty())
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::MappingEditBarrier { .. }
    ));
    dom.in_scope(ScopeId::APP, || {
        let view = consume_context::<crate::frame::view_state::ViewState>();
        assert_eq!(
            *view.selected_mapping.peek(),
            Some(("Default".into(), address(InputId::Button { index: 3 })))
        );
    });
    assert!(!dioxus_ssr::render(&dom).contains("if-add-inline--pad"));
    assert!(
        f.state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&address(InputId::Button { index: 3 }), "Default")
            .is_none()
    );
}
#[test]
fn failed_actual_add_button_retains_pad_selection_and_history() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (mut dom, f, rx) = captured();
    drop(rx);
    let before = dom.in_scope(ScopeId::APP, || {
        consume_context::<crate::frame::view_state::ViewState>()
            .selected_mapping
            .peek()
            .clone()
    });
    click(&dom, "Add");
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("if-add-inline--pad"));
    assert!(dioxus_ssr::render(&dom).contains(r#"role="alert""#));
    dom.in_scope(ScopeId::APP, || {
        assert_eq!(
            *consume_context::<crate::frame::view_state::ViewState>()
                .selected_mapping
                .peek(),
            before
        );
    });
    assert_eq!(f.editor.borrow().unwrap().undo_log.peek().stacks.len(), 0);
}

#[test]
fn actual_collision_renders_warning_and_opens_existing_without_dispatch() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (mut dom, _, rx) = captured_input(0);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("if-add-inline--collision"));
    assert!(html.find("if-badge--warning").unwrap() < html.find("already mapped to").unwrap());
    click(&dom, "Edit existing");
    dom.render_immediate_to_vec();
    assert!(!dioxus_ssr::render(&dom).contains("if-add-inline--collision"));
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
}
#[test]
fn actual_enter_commit_uses_the_same_dispatch_and_navigation_path() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (mut dom, _, rx) = captured();
    let id = element(&dom, dom.base_scope().root_node(), "Add").unwrap();
    let keyboard = SerializedKeyboardData::new(
        Key::Enter,
        Code::Enter,
        Location::Standard,
        false,
        Modifiers::empty(),
        false,
    );
    dom.runtime().handle_event(
        "keydown",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(keyboard))) as Rc<dyn std::any::Any>,
            true,
        ),
        id,
    );
    dom.render_immediate_to_vec();
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::SetMapping { .. }
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::MappingEditBarrier { .. }
    ));
    assert!(!dioxus_ssr::render(&dom).contains("if-add-inline--pad"));
}
