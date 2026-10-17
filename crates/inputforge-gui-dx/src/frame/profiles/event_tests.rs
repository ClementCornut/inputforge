//! Real form handlers retain failed drafts and surface accessible feedback.
use crate::{
    context::{AppContext, ConfigSnapshot, LiveSnapshot, SettingsSnapshot},
    profiles::{Context, Intent, View},
};
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, state::AppState};
use parking_lot::RwLock;
use std::{
    rc::Rc,
    sync::{Arc, mpsc},
};
fn root() -> Element {
    let state = Arc::new(RwLock::new(AppState::new()));
    let (commands, rx) = mpsc::channel::<EngineCommand>();
    drop(rx);
    let c = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&state), commands.clone())
    });
    let pv = use_signal(View::default);
    let owner = use_hook(|| {
        let o = c.mount(pv);
        let _ = c.dispatch(o, Intent::New);
        o
    });
    use_context_provider(|| Context {
        view: pv,
        owner,
        workflow: 0,
        bindings: c.bindings(),
    });
    let meta = use_signal(Default::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| {
        crate::test_support::provide_engine(state, commands);
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    let view = crate::frame::use_view_state_provider(meta);
    use_context_provider(|| view);
    rsx! { super::new_profile_submode::NewProfileSubMode {} }
}
pub(super) fn listeners(
    dom: &VirtualDom,
    node: &VNode,
    name: &str,
    ids: &mut Vec<dioxus::core::ElementId>,
) {
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
#[test]
fn failed_create_keeps_text_and_renders_alert_through_real_handlers() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    let mut ids = vec![];
    listeners(&dom, dom.base_scope().root_node(), "input", &mut ids);
    dom.runtime().handle_event(
        "input",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                "Retain this profile".into(),
                vec![],
            )))) as Rc<dyn std::any::Any>,
            false,
        ),
        ids[0],
    );
    dom.render_immediate_to_vec();
    let mut ids = vec![];
    listeners(&dom, dom.base_scope().root_node(), "click", &mut ids);
    dom.runtime().handle_event(
        "click",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(
                SerializedMouseData::default(),
            ))) as Rc<dyn std::any::Any>,
            false,
        ),
        ids[1],
    );
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Retain this profile"), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
}

#[derive(Clone)]
struct RowFixture {
    delete: bool,
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    controller: std::cell::RefCell<Option<inputforge_app::profiles::Controller>>,
    view: std::cell::RefCell<Option<Signal<View>>>,
    owner: std::cell::Cell<u64>,
}
fn row_root() -> Element {
    let f = use_context::<Rc<RowFixture>>();
    let c = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&f.state), f.commands.clone())
    });
    *f.controller.borrow_mut() = Some(c.clone());
    let mut bindings = c.bindings();
    let fixture = Rc::clone(&f);
    bindings.mount = Callback::new(move |v: Signal<View>| {
        let owner = c.mount(v);
        let t = v.peek().rows[0].target.clone();
        let _ = c.dispatch(
            owner,
            if fixture.delete {
                Intent::BeginDelete(t)
            } else {
                Intent::BeginRename(t)
            },
        );
        fixture.owner.set(owner);
        *fixture.view.borrow_mut() = Some(v);
        owner
    });
    use_context_provider(|| bindings);
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
    rsx! {super::ProfilesPanel {}}
}
fn keyboard(dom: &VirtualDom, key: Key, code: Code) {
    let mut ids = vec![];
    listeners(dom, dom.base_scope().root_node(), "keydown", &mut ids);
    let data = SerializedKeyboardData::new(
        key,
        code,
        Location::Standard,
        false,
        Modifiers::empty(),
        false,
    );
    dom.runtime().handle_event(
        "keydown",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
            false,
        ),
        ids[1],
    );
}
#[test]
fn failed_inline_rename_keeps_draft_after_same_path_replacement_and_escape_cancels() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let profile = || {
        inputforge_core::profile::Profile::new(
            "Alpha".into(),
            vec![],
            inputforge_core::mode::Modes::new(vec!["Default".into()]).unwrap(),
            vec![],
            vec![],
            "Default".into(),
        )
    };
    let mut s = AppState::with_profile(profile());
    s.profile_path = Some("/profiles/Alpha.toml".into());
    let (commands, rx) = mpsc::channel();
    drop(rx);
    let f = Rc::new(RowFixture {
        delete: false,
        state: Arc::new(RwLock::new(s)),
        commands,
        controller: std::cell::RefCell::default(),
        view: std::cell::RefCell::default(),
        owner: std::cell::Cell::new(0),
    });
    let mut dom = VirtualDom::new(row_root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    let mut ids = vec![];
    listeners(&dom, dom.base_scope().root_node(), "input", &mut ids);
    dom.runtime().handle_event(
        "input",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                "Retain rename".into(),
                vec![],
            )))) as Rc<dyn std::any::Any>,
            false,
        ),
        ids[1],
    );
    dom.render_immediate_to_vec();
    keyboard(&dom, Key::Enter, Code::Enter);
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Retain rename"), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
    f.state.write().active_profile = Some(profile());
    f.controller
        .borrow()
        .as_ref()
        .unwrap()
        .refresh(f.owner.get());
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("value=\"Retain rename\""), "{html}");
    let mut ids = vec![];
    listeners(&dom, dom.base_scope().root_node(), "blur", &mut ids);
    dom.runtime().handle_event(
        "blur",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedFocusData {})))
                as Rc<dyn std::any::Any>,
            false,
        ),
        ids[1],
    );
    dom.render_immediate_to_vec();
    assert!(f.view.borrow().unwrap().peek().rename.is_some());
    keyboard(&dom, Key::Escape, Code::Escape);
    dom.render_immediate_to_vec();
    assert!(f.view.borrow().unwrap().peek().rename.is_none());
}

#[test]
fn failed_delete_handler_keeps_confirmation_until_explicit_cancel() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut s = AppState::with_profile(inputforge_core::profile::Profile::new(
        "Alpha".into(),
        vec![],
        inputforge_core::mode::Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    s.profile_path = Some("/profiles/Alpha.toml".into());
    let (commands, rx) = mpsc::channel();
    drop(rx);
    let f = Rc::new(RowFixture {
        delete: true,
        state: Arc::new(RwLock::new(s)),
        commands,
        controller: std::cell::RefCell::default(),
        view: std::cell::RefCell::default(),
        owner: std::cell::Cell::new(0),
    });
    let mut dom = VirtualDom::new(row_root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    let mut ids = vec![];
    listeners(&dom, dom.base_scope().root_node(), "click", &mut ids);
    let mouse = || {
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(
                SerializedMouseData::default(),
            ))) as Rc<dyn std::any::Any>,
            false,
        )
    };
    dom.runtime()
        .handle_event("click", mouse(), *ids.last().unwrap());
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("This is the active profile"), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
    assert!(f.view.borrow().unwrap().peek().delete.is_some());
    let mut ids = vec![];
    listeners(&dom, dom.base_scope().root_node(), "click", &mut ids);
    dom.runtime()
        .handle_event("click", mouse(), ids[ids.len() - 2]);
    dom.render_immediate_to_vec();
    assert!(f.view.borrow().unwrap().peek().delete.is_none());
    assert!(!dioxus_ssr::render(&dom).contains("Delete profile"));
}
