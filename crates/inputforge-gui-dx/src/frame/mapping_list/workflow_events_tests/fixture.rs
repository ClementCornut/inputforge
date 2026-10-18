use crate::{
    context::{AppContext, ConfigSnapshot, LiveSnapshot, SettingsSnapshot},
    frame::view_state::ViewState,
    patterns::live_capture::use_live_capture_provider,
};
use dioxus::{
    core::{AttributeValue, DynamicNode, ElementId, TemplateNode},
    prelude::*,
};
use inputforge_core::{
    action::{Action, Mapping},
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::{DeviceId, InputAddress, InputId},
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

pub(super) fn address(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("rail".into()),
        input: InputId::Button { index },
    }
}
pub(super) struct Fixture {
    pub state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    pub config: RefCell<Option<Signal<ConfigSnapshot>>>,
    pub view: RefCell<Option<ViewState>>,
}
fn root() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let frame = inputforge_app::bridge::prepare(&f.state.read(), None);
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| {
        crate::test_support::provide_engine(Arc::clone(&f.state), f.commands.clone());
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    let view = crate::frame::use_view_state_provider(meta);
    use_context_provider(|| view);
    let controller = use_hook(|| {
        inputforge_app::mapping_edits::Controller::new(Arc::clone(&f.state), f.commands.clone())
    });
    let bindings = use_hook(|| controller.bindings());
    use_context_provider(|| bindings);
    crate::frame::mapping_editor::use_editor_state_provider();
    use_live_capture_provider();
    *f.config.borrow_mut() = Some(config);
    *f.view.borrow_mut() = Some(view);
    rsx! { super::super::MappingList {} }
}
pub(super) fn mount() -> (VirtualDom, Rc<Fixture>, mpsc::Receiver<EngineCommand>) {
    mount_with_document(Rc::new(document::NoOpDocument))
}
pub(super) fn mount_with_document(
    document: Rc<dyn document::Document>,
) -> (VirtualDom, Rc<Fixture>, mpsc::Receiver<EngineCommand>) {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let profile = Profile::new(
        "Rail".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Other".into()]).unwrap(),
        (0..3)
            .map(|i| Mapping {
                input: address(i),
                mode: "Default".into(),
                name: Some(["First", "Second", "Third"][usize::from(i)].into()),
                actions: vec![Action::Invert],
            })
            .collect(),
        vec![],
        "Default".into(),
    );
    let (commands, rx) = mpsc::channel();
    let f = Rc::new(Fixture {
        state: Arc::new(RwLock::new(AppState::with_profile(profile))),
        commands,
        config: RefCell::default(),
        view: RefCell::default(),
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.provide_root_context(document);
    dom.rebuild_in_place();
    (dom, f, rx)
}
pub(super) fn menu(dom: &mut VirtualDom, label: &str) {
    let id = element_in(dom, dom.base_scope().root_node(), label, true).unwrap();
    dom.runtime().handle_event(
        "contextmenu",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(
                SerializedMouseData::default(),
            ))) as Rc<dyn std::any::Any>,
            false,
        ),
        id,
    );
    dom.render_immediate_to_vec();
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
                && matches!(current, TemplateNode::Element { tag: "div", attrs, .. } if attrs.iter().any(|attr| matches!(attr, dioxus::core::TemplateAttribute::Static { name: "role", value: "group", .. }))))
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

pub(super) fn elements(dom: &VirtualDom, node: &VNode, class: &str) -> Vec<ElementId> {
    let mut found = vec![];
    for (index, path) in node.template.attr_paths.iter().enumerate() {
        let mut current = &node.template.roots[usize::from(path[0])];
        for offset in &path[1..] {
            if let TemplateNode::Element { children, .. } = current {
                current = &children[usize::from(*offset)];
            }
        }
        if let TemplateNode::Element { attrs, .. } = current
            && (attrs.iter().any(|a| matches!(a, dioxus::core::TemplateAttribute::Static { name: "class", value, .. } if *value == class)) || node.dynamic_attrs[index].iter().any(|a| a.name == "class" && matches!(&a.value, AttributeValue::Text(value) if value == class)))
            && let Some(id) = node.mounted_dynamic_attribute(index, dom) && !found.contains(&id) { found.push(id); }
    }
    for (i, child) in node.dynamic_nodes.iter().enumerate() {
        match child {
            DynamicNode::Component(c) => {
                if let Some(s) = c.mounted_scope(i, node, dom) {
                    found.extend(elements(dom, s.root_node(), class));
                }
            }
            DynamicNode::Fragment(nodes) => {
                for n in nodes {
                    found.extend(elements(dom, n, class));
                }
            }
            _ => {}
        }
    }
    found
}
pub(super) fn drag(dom: &VirtualDom, name: &str, id: ElementId) {
    let data: SerializedDragData = serde_json::from_value(serde_json::json!({
            "mouse": SerializedMouseData::default(),
            "data_transfer": { "items": [], "files": [], "effect_allowed": "move", "drop_effect": "move" },
        })).unwrap();
    dom.runtime().handle_event(
        name,
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
            true,
        ),
        id,
    );
}
