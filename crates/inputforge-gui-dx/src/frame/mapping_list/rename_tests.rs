//! The rail preserves its captured name when config advances before commit.
use super::rename_inline::run_commit;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, SettingsSnapshot};
use dioxus::prelude::*;
use inputforge_core::{
    action::Action, engine::EngineCommand, mode::Modes, profile::Profile, state::AppState,
    types::InputAddress,
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
struct Fixture {
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    handler: RefCell<Option<Callback<String, Result<(), String>>>>,
    config: RefCell<Option<Signal<ConfigSnapshot>>>,
    renaming: RefCell<Option<Signal<Option<InputAddress>>>>,
}
fn root() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let frame = inputforge_app::bridge::prepare(&f.state.read(), None);
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| AppContext {
        state: Arc::clone(&f.state),
        commands: f.commands.clone(),
        meta,
        config,
        live,
        settings,
    });
    let view = crate::frame::use_view_state_provider(meta);
    use_context_provider(|| view);
    crate::frame::mapping_editor::use_editor_state_provider();
    let summary = config.read().mappings[0].clone();
    let renaming = use_signal(|| Some(InputAddress::Unbound));
    crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::use_origin();
    let captured = use_hook(|| super::rename_inline::capture(&summary));
    let handler = Callback::new(move |raw: String| run_commit(&raw, captured.as_ref(), renaming));
    *f.renaming.borrow_mut() = Some(renaming);
    *f.handler.borrow_mut() = Some(handler);
    *f.config.borrow_mut() = Some(config);
    rsx! { if renaming.read().is_some() { super::rename_inline::RenameInline { summary, state: renaming } } }
}
#[test]
fn rail_commit_rejects_old_name_even_when_the_current_config_has_advanced() {
    for stale in [false, true] {
        let profile = Profile::new(
            "Rail".into(),
            vec![],
            Modes::new(vec!["Default".into()]).unwrap(),
            vec![inputforge_core::action::Mapping {
                input: InputAddress::Unbound,
                mode: "Default".into(),
                name: None,
                actions: vec![Action::Invert],
            }],
            vec![],
            "Default".into(),
        );
        let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
        let (commands, rx) = mpsc::channel();
        let f = Rc::new(Fixture {
            state,
            commands,
            handler: RefCell::default(),
            config: RefCell::default(),
            renaming: RefCell::default(),
        });
        let mut dom = VirtualDom::new(root);
        dom.provide_root_context(Rc::clone(&f));
        dom.rebuild_in_place();
        if stale {
            f.state
                .write()
                .active_profile
                .as_mut()
                .unwrap()
                .set_mapping(
                    &InputAddress::Unbound,
                    "Default",
                    Some("External".into()),
                    vec![Action::Invert],
                );
            f.config
                .borrow()
                .unwrap()
                .set(inputforge_app::bridge::prepare(&f.state.read(), None).config);
        }
        let result = f.handler.borrow().unwrap().call("Requested".into());
        assert_eq!(result.is_err(), stale);
        if stale {
            assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
            assert_eq!(
                *f.renaming.borrow().unwrap().peek(),
                Some(InputAddress::Unbound),
                "rejected rename must retain the draft"
            );
        } else {
            assert!(
                matches!(rx.try_recv().unwrap(), EngineCommand::SetMapping { name, actions, .. } if name.as_deref() == Some("Requested") && actions == vec![Action::Invert])
            );
        }
    }
}

fn listener(dom: &VirtualDom, node: &VNode, name: &str) -> Option<dioxus::core::ElementId> {
    use dioxus::core::DynamicNode;
    for (i, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs
            .iter()
            .any(|a| a.name == name || a.name == format!("on{name}"))
        {
            return node.mounted_dynamic_attribute(i, dom);
        }
    }
    node.dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(i, child)| match child {
            DynamicNode::Component(c) => c
                .mounted_scope(i, node, dom)
                .and_then(|s| listener(dom, s.root_node(), name)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|n| listener(dom, n, name)),
            _ => None,
        })
}
fn form(dom: &VirtualDom, value: &str) {
    let id = listener(dom, dom.base_scope().root_node(), "input").unwrap();
    let data = SerializedFormData::new(value.into(), vec![]);
    dom.runtime().handle_event(
        "input",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
            false,
        ),
        id,
    );
}
fn keyboard(dom: &VirtualDom, key: Key, code: Code) {
    let id = listener(dom, dom.base_scope().root_node(), "keydown").unwrap();
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
        id,
    );
}
#[test]
fn failed_enter_and_blur_retain_text_and_show_feedback_then_escape_cancels_offline() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let profile = Profile::new(
        "Rail".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![inputforge_core::action::Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: None,
            actions: vec![Action::Invert],
        }],
        vec![],
        "Default".into(),
    );
    let (commands, rx) = mpsc::channel();
    let f = Rc::new(Fixture {
        state: Arc::new(RwLock::new(AppState::with_profile(profile))),
        commands,
        handler: RefCell::default(),
        config: RefCell::default(),
        renaming: RefCell::default(),
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    drop(rx);
    form(&dom, "Retain this draft");
    dom.render_immediate_to_vec();
    keyboard(&dom, Key::Enter, Code::Enter);
    dom.render_immediate_to_vec();
    for blur in [false, true] {
        if blur {
            let id = listener(&dom, dom.base_scope().root_node(), "focusout").unwrap();
            dom.runtime().handle_event(
                "focusout",
                Event::new(
                    Rc::new(PlatformEventData::new(Box::new(SerializedFocusData {})))
                        as Rc<dyn std::any::Any>,
                    false,
                ),
                id,
            );
            dom.render_immediate_to_vec();
        }
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Retain this draft"), "{html}");
        assert!(html.contains("role=\"alert\""), "{html}");
        assert!(f.renaming.borrow().unwrap().peek().is_some());
    }
    f.state.write().session.offline = true;
    keyboard(&dom, Key::Escape, Code::Escape);
    dom.render_immediate_to_vec();
    assert!(f.renaming.borrow().unwrap().peek().is_none());
}

#[test]
fn retained_rail_draft_rejects_same_key_replacement_after_props_rerender() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let profile = Profile::new(
        "Rail".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![inputforge_core::action::Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: None,
            actions: vec![Action::Invert],
        }],
        vec![],
        "Default".into(),
    );
    let (commands, rx) = mpsc::channel();
    let f = Rc::new(Fixture {
        state: Arc::new(RwLock::new(AppState::with_profile(profile))),
        commands,
        handler: RefCell::default(),
        config: RefCell::default(),
        renaming: RefCell::default(),
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    form(&dom, "Original draft");
    dom.render_immediate_to_vec();
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(
            &InputAddress::Unbound,
            "Default",
            None,
            vec![Action::Deadzone {
                config: inputforge_core::processing::DeadzoneConfig::default(),
            }],
        );
    f.config
        .borrow()
        .unwrap()
        .set(inputforge_app::bridge::prepare(&f.state.read(), None).config);
    dom.render_immediate_to_vec();
    keyboard(&dom, Key::Enter, Code::Enter);
    dom.render_immediate_to_vec();
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Original draft"), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
    assert!(f.renaming.borrow().unwrap().peek().is_some());
}

#[test]
fn unavailable_rail_target_cannot_become_an_empty_replacement_after_rerender() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let profile = Profile::new(
        "Rail".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![inputforge_core::action::Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: None,
            actions: vec![Action::Invert],
        }],
        vec![],
        "Default".into(),
    );
    let (commands, rx) = mpsc::channel();
    let mut state = AppState::with_profile(profile);
    state.session.offline = true;
    let f = Rc::new(Fixture {
        state: Arc::new(RwLock::new(state)),
        commands,
        handler: RefCell::default(),
        config: RefCell::default(),
        renaming: RefCell::default(),
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    form(&dom, "Unavailable target draft");
    dom.render_immediate_to_vec();
    {
        let mut state = f.state.write();
        state.session.offline = false;
        state.active_profile.as_mut().unwrap().set_mapping(
            &InputAddress::Unbound,
            "Default",
            None,
            vec![],
        );
    };
    f.config
        .borrow()
        .unwrap()
        .set(inputforge_app::bridge::prepare(&f.state.read(), None).config);
    dom.render_immediate_to_vec();
    keyboard(&dom, Key::Enter, Code::Enter);
    dom.render_immediate_to_vec();
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Unavailable target draft"), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
    assert!(f.renaming.borrow().unwrap().peek().is_some());
    let editor = dom.in_scope(
        ScopeId::APP,
        consume_context::<crate::frame::mapping_editor::EditorState>,
    );
    assert!(editor.undo_log.peek().stacks.is_empty());
}
