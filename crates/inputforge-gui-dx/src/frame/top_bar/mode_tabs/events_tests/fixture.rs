use crate::{
    context::{AppContext, LiveSnapshot, SettingsSnapshot},
    frame::view_state::ViewState,
};
use dioxus::{
    core::{AttributeValue, DynamicNode, ElementId, TemplateNode},
    prelude::*,
};
use inputforge_core::{engine::EngineCommand, mode::Modes, profile::Profile, state::AppState};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
pub(super) struct Fixture {
    pub state: Arc<RwLock<AppState>>,
    tx: mpsc::Sender<EngineCommand>,
    pub view: RefCell<Option<ViewState>>,
    pub focus: RefCell<Option<Signal<Option<String>>>>,
    pub meta: RefCell<Option<Signal<crate::context::MetaSnapshot>>>,
}
fn root() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let frame = inputforge_app::bridge::prepare(&f.state.read(), None);
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| {
        crate::test_support::provide_engine(Arc::clone(&f.state), f.tx.clone());
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    let view = crate::frame::use_view_state_provider(meta);
    use_context_provider(|| view);
    crate::frame::mapping_editor::use_editor_state_provider();
    let target = use_signal(|| None);
    let focus = use_signal(|| None);
    crate::frame::top_bar::mode_tabs::provide_publication(target, focus);
    *f.view.borrow_mut() = Some(view);
    *f.focus.borrow_mut() = Some(focus);
    *f.meta.borrow_mut() = Some(meta);
    rsx! { super::super::ModeTabs {} super::super::ModeDeleteDialog {} }
}
pub(super) fn mount() -> (VirtualDom, Rc<Fixture>, mpsc::Receiver<EngineCommand>) {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let profile = Profile::new(
        "P".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Combat".into(), "Landing".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    let (tx, rx) = mpsc::channel();
    let f = Rc::new(Fixture {
        state: Arc::new(RwLock::new(AppState::with_profile(profile))),
        tx,
        view: RefCell::default(),
        focus: RefCell::default(),
        meta: RefCell::default(),
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.provide_root_context(Rc::new(document::NoOpDocument) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    settle(&mut dom);
    (dom, f, rx)
}
pub(super) fn settle(dom: &mut VirtualDom) {
    for _ in 0..4 {
        dom.render_immediate_to_vec();
    }
}
pub(super) fn find(dom: &VirtualDom, node: &VNode, name: &str, value: &str) -> Option<ElementId> {
    for (i, path) in node.template.attr_paths.iter().enumerate() {
        let mut current = &node.template.roots[usize::from(path[0])];
        for offset in &path[1..] {
            if let TemplateNode::Element { children, .. } = current {
                current = &children[usize::from(*offset)];
            }
        }
        if let TemplateNode::Element { tag, attrs, .. } = current {
            let matches = (name == "tag" && *tag == value) || attrs.iter().any(|a| matches!(a, dioxus::core::TemplateAttribute::Static { name: n, value: v, .. } if *n == name && *v == value)) || node.dynamic_attrs[i].iter().any(|a| a.name == name && matches!(&a.value, AttributeValue::Text(v) if v == value));
            if matches {
                return node.mounted_dynamic_attribute(i, dom);
            }
        }
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(i, child)| match child {
            DynamicNode::Component(c) => c
                .mounted_scope(i, node, dom)
                .and_then(|s| find(dom, s.root_node(), name, value)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|n| find(dom, n, name, value)),
            _ => None,
        })
}
pub(super) fn event(
    dom: &VirtualDom,
    event: &str,
    name: &str,
    value: &str,
    data: impl std::any::Any,
) {
    let id = find(dom, dom.base_scope().root_node(), name, value)
        .unwrap_or_else(|| panic!("missing {name}={value}: {}", dioxus_ssr::render(dom)));
    dom.runtime().handle_event(
        event,
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
            true,
        ),
        id,
    );
}
pub(super) fn click(dom: &VirtualDom, label: &str) {
    // Menu items have literal text, so resolve their button through its component subtree.
    fn button(dom: &VirtualDom, n: &VNode, label: &str) -> Option<ElementId> {
        if n.template.roots.iter().any(|root| text(root, n, label))
            && let Some(id) = find(dom, n, "tag", "button")
        {
            return Some(id);
        }
        n.dynamic_nodes
            .iter()
            .enumerate()
            .find_map(|(i, c)| match c {
                DynamicNode::Component(c) => c
                    .mounted_scope(i, n, dom)
                    .and_then(|s| button(dom, s.root_node(), label)),
                DynamicNode::Fragment(ns) => ns.iter().find_map(|n| button(dom, n, label)),
                _ => None,
            })
    }
    fn text(t: &TemplateNode, n: &VNode, label: &str) -> bool {
        match t {
            TemplateNode::Text { text } => *text == label,
            TemplateNode::Element { children, .. } => children.iter().any(|c| text(c, n, label)),
            TemplateNode::Dynamic { id } => match &n.dynamic_nodes[*id] {
                DynamicNode::Text(t) => t.value == label,
                DynamicNode::Fragment(ns) => ns
                    .iter()
                    .any(|n| n.template.roots.iter().any(|t| text(t, n, label))),
                _ => false,
            },
        }
    }
    let id = find(dom, dom.base_scope().root_node(), "aria-label", label)
        .or_else(|| button(dom, dom.base_scope().root_node(), label))
        .unwrap_or_else(|| panic!("missing {label}: {}", dioxus_ssr::render(dom)));
    dom.runtime().handle_event(
        "click",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(
                SerializedMouseData::default(),
            ))) as Rc<dyn std::any::Any>,
            true,
        ),
        id,
    );
}
pub(super) fn input(dom: &VirtualDom, value: &str) {
    event(
        dom,
        "input",
        "tag",
        "input",
        SerializedFormData::new(value.into(), vec![]),
    );
}
pub(super) fn enter(dom: &VirtualDom) {
    let data: SerializedKeyboardData = serde_json::from_value(serde_json::json!({"key":"Enter", "code":"Enter", "location":0, "repeat":false, "modifiers":0, "is_composing":false, "char_code":13, "key_code":13, "which":13, "alt_key":false, "ctrl_key":false, "meta_key":false, "shift_key":false})).unwrap();
    event(dom, "keydown", "tag", "input", data);
}
pub(super) fn menu(dom: &mut VirtualDom) {
    event(
        dom,
        "contextmenu",
        "data-mode",
        "Combat",
        SerializedMouseData::default(),
    );
    settle(dom);
}
