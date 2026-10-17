use crate::{
    context::AppContext,
    frame::view_state::{MainSurface, ViewState},
};
use dioxus::{
    core::{AttributeValue, DynamicNode, ElementId, TemplateNode},
    prelude::*,
};
use inputforge_core::{engine::EngineCommand, state::AppState};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

pub(super) struct Fixture {
    pub state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    pub view: RefCell<Option<ViewState>>,
    pub toasts: RefCell<Option<Signal<Vec<crate::toast::Toast>>>>,
}
fn root() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let prepared = inputforge_app::bridge::prepare(&f.state.read(), None);
    let meta = use_signal(|| prepared.meta);
    let config = use_signal(|| prepared.config);
    let live = use_signal(|| prepared.live);
    let settings = use_signal(|| prepared.settings);
    let ctx = {
        crate::test_support::provide_engine(Arc::clone(&f.state), f.commands.clone());
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    };
    use_context_provider(|| ctx.clone());
    let mut view = crate::frame::use_view_state_provider(meta);
    use_hook(|| view.main_surface.set(MainSurface::BulkMap));
    use_context_provider(|| view);
    let controller = use_hook(|| {
        inputforge_app::mapping_edits::Controller::new(Arc::clone(&f.state), f.commands.clone())
    });
    let bindings = use_hook(|| controller.bindings());
    use_context_provider(|| bindings);
    crate::frame::mapping_editor::use_editor_state_provider();
    let toasts = inputforge_app::notifications::use_provider().visible;
    *f.view.borrow_mut() = Some(view);
    *f.toasts.borrow_mut() = Some(toasts);
    rsx! { super::super::BulkMapPanel {} }
}
pub(super) fn mount() -> (VirtualDom, Rc<Fixture>, mpsc::Receiver<EngineCommand>) {
    mount_state(super::super::tests::seeded_state(true))
}
pub(super) fn mount_state(
    state: AppState,
) -> (VirtualDom, Rc<Fixture>, mpsc::Receiver<EngineCommand>) {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let (commands, rx) = mpsc::channel();
    let f = Rc::new(Fixture {
        state: Arc::new(RwLock::new(state)),
        commands,
        view: RefCell::default(),
        toasts: RefCell::default(),
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    (dom, f, rx)
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

pub(super) fn change(dom: &VirtualDom, id: &str, value: &str) {
    let element = by_id(dom, dom.base_scope().root_node(), id)
        .unwrap_or_else(|| panic!("missing batch field {id}"));
    let data = SerializedFormData::new(value.into(), vec![]);
    dom.runtime().handle_event(
        "change",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
            false,
        ),
        element,
    );
}
fn by_id(dom: &VirtualDom, node: &VNode, id: &str) -> Option<ElementId> {
    for (index, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs.iter().any(|a| {
            a.name == "id" && matches!(&a.value, AttributeValue::Text(value) if value == id)
        }) {
            return node.mounted_dynamic_attribute(index, dom);
        }
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(i, child)| match child {
            DynamicNode::Component(c) => c
                .mounted_scope(i, node, dom)
                .and_then(|s| by_id(dom, s.root_node(), id)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|n| by_id(dom, n, id)),
            _ => None,
        })
}
