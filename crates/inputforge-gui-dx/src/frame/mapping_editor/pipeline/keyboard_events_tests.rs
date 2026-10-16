//! Both rendered keyboard surfaces retain their existing dispatch and undo handlers.
use super::{add_palette::AddPalette, stage_body::StageBody};
use crate::keyboard_document as fake_document;
use crate::{
    context::{AppContext, LiveSnapshot, SettingsSnapshot},
    frame::mapping_editor::{
        EditorState,
        undo_log::{StageId, StageIdSegment},
        use_editor_state_provider,
    },
    patterns::keyboard_capture::use_keyboard_capture_provider,
};
use dioxus::{
    core::{AttributeValue, DynamicNode, ElementId, TemplateNode},
    html::SerializedHtmlEventConverter,
    prelude::*,
};
use inputforge_core::{
    action::{Action, Mapping, OutputBehavior},
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::{InputAddress, KeyCombo, PhysicalKey},
};
use parking_lot::RwLock;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, mpsc},
};
struct Fixture {
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    add: bool,
    menu: Cell<bool>,
    meta: RefCell<Option<Signal<crate::context::MetaSnapshot>>>,
    editor: RefCell<Option<EditorState>>,
}
fn original() -> Action {
    Action::MapToKeyboard {
        key: KeyCombo {
            key: PhysicalKey::F11,
            modifiers: vec![],
        },
        behavior: OutputBehavior::Hold,
    }
}
fn harness() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let key = ("Default".into(), InputAddress::Unbound);
    let frame = inputforge_app::bridge::prepare(&f.state.read(), Some(&key));
    let meta = use_signal(|| frame.meta);
    *f.meta.borrow_mut() = Some(meta);
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
    let mut view = crate::frame::use_view_state_provider(meta);
    use_hook(|| view.selected_mapping.set(Some(key.clone())));
    use_context_provider(|| view);
    let editor = use_editor_state_provider();
    *f.editor.borrow_mut() = Some(editor);
    use_keyboard_capture_provider();
    if f.menu.get() {
        rsx! { super::stage_actions_menu::StageActionsMenu { mapping_key: key, root_actions: vec![original()] } }
    } else if f.add {
        rsx! { AddPalette { mapping_key: key, path_prefix: vec![], target_len: 1, root_actions: vec![original()], louder: true } }
    } else {
        rsx! { StageBody { mapping_key: key, stage_id: StageId(vec![StageIdSegment::Index(0)]), action: original(), root_actions: vec![original()] } }
    }
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
        .unwrap_or_else(|| panic!("missing keyboard control {label}"));
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
#[test]
fn keyboard_buttons_dispatch_once_preserve_names_and_record_undo_after_delivery() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            for add in [false, true] {
                for failed in [false, true] {
                    let (mut dom, f, document, received) = fixture(add, failed);
                    if add {
                        click(&dom, "Add first stage");
                        dom.render_immediate_to_vec();
                        click(&dom, "Map to keyboard");
                    } else {
                        click(&dom, "Capture keyboard shortcut");
                    }
                    dom.render_immediate_to_vec();
                    assert!(dioxus_ssr::render(&dom).contains("is-listening"));
                    document.key(0, "keydown", "F12", "F12");
                    document.key(0, "keydown", "F12", "F12");
                    for _ in 0..4 {
                        tokio::task::yield_now().await;
                        dom.render_immediate_to_vec();
                    }
                    let editor = f.editor.borrow().unwrap();
                    if let Some(received) = received {
                        let EngineCommand::SetMapping {
                            input,
                            mode,
                            name,
                            actions,
                        } = received.try_recv().unwrap()
                        else {
                            panic!("completion must retain the mapping handler");
                        };
                        assert_eq!(input, InputAddress::Unbound);
                        assert_eq!(mode, "Default");
                        assert_eq!(name.as_deref(), Some("Fixture"));
                        assert_eq!(actions.len(), if add { 2 } else { 1 });
                        assert!(matches!(
                            actions.last(),
                            Some(Action::MapToKeyboard {
                                key: KeyCombo {
                                    key: PhysicalKey::F12,
                                    ..
                                },
                                behavior: OutputBehavior::Hold
                            })
                        ));
                        assert_eq!(
                            editor.undo_log.peek().stacks
                                [&("Default".into(), InputAddress::Unbound)]
                                .undo
                                .len(),
                            1
                        );
                        expect_fence(&received);
                        assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
                        if !add {
                            assert_eq!(
                                editor.undo_log.peek().stacks
                                    [&("Default".into(), InputAddress::Unbound)]
                                    .undo[0]
                                    .label,
                                "Map to keyboard: key F11 -> F12"
                            );
                            assert!(
                                dioxus_ssr::render(&dom).contains("F12"),
                                "stale props must not restore the old key"
                            );
                        }
                    } else {
                        assert_eq!(editor.undo_log.peek().stacks.len(), 0);
                        if !add {
                            assert!(
                                dioxus_ssr::render(&dom).contains("F11"),
                                "failed dispatch must preserve the saved key"
                            );
                        }
                    }
                    assert!(!dioxus_ssr::render(&dom).contains("is-listening"));
                    assert_eq!(
                        f.state
                            .read()
                            .active_profile
                            .as_ref()
                            .unwrap()
                            .find_mapping(&InputAddress::Unbound, "Default")
                            .unwrap()
                            .actions,
                        vec![original()]
                    );
                    drop(dom);
                    assert_eq!(document.shutdowns(), 1);
                    assert_eq!(document.cleanups(), 1);
                }
            }
        });
}

#[test]
fn keyboard_behavior_button_records_old_and_new_behavior_in_undo_hint() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (mut dom, f, _, received) = fixture(false, false);
    click(&dom, "Pulse");
    dom.render_immediate_to_vec();
    assert!(
        matches!(received.unwrap().try_recv().unwrap(), EngineCommand::SetMapping { actions, .. }
        if matches!(actions[0], Action::MapToKeyboard { behavior: OutputBehavior::Pulse, .. }))
    );
    assert_eq!(
        f.editor.borrow().unwrap().undo_log.peek().stacks
            [&("Default".into(), InputAddress::Unbound)]
            .undo[0]
            .label,
        "Map to keyboard: behavior Hold -> Pulse"
    );
}

fn fixture(
    add: bool,
    failed: bool,
) -> (
    VirtualDom,
    Rc<Fixture>,
    Rc<fake_document::Document>,
    Option<mpsc::Receiver<EngineCommand>>,
) {
    let profile = Profile::new(
        "Keyboard".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: Some("Fixture".into()),
            actions: vec![original()],
        }],
        vec![],
        "Default".into(),
    );
    let mut state = AppState::with_profile(profile);
    state.session.keyboard_supported = true;
    let (commands, received) = mpsc::channel();
    let received = if failed {
        drop(received);
        None
    } else {
        Some(received)
    };
    let f = Rc::new(Fixture {
        state: Arc::new(RwLock::new(state)),
        commands,
        add,
        menu: Cell::new(false),
        meta: RefCell::default(),
        editor: RefCell::new(None),
    });
    let document = Rc::new(fake_document::Document::default());
    let mut dom = VirtualDom::new(harness);
    dom.provide_root_context(Rc::clone(&f));
    dom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    (dom, f, document, received)
}

fn expect_fence(received: &mpsc::Receiver<EngineCommand>) {
    assert!(matches!(
        received.try_recv().unwrap(),
        EngineCommand::MappingEditBarrier { .. }
    ));
}

#[test]
fn palette_delivery_failure_retains_the_menu_and_success_closes_it() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for failed in [true, false] {
        let (mut dom, f, _, received) = fixture(true, failed);
        click(&dom, "Add first stage");
        dom.render_immediate_to_vec();
        click(&dom, "Invert");
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(html.contains("aria-expanded=\"true\""), failed, "{html}");
        let editor = f.editor.borrow().unwrap();
        assert_eq!(editor.error.peek().is_some(), failed);
        if let Some(received) = received {
            assert!(
                matches!(received.try_recv().unwrap(), EngineCommand::SetMapping { actions, .. } if actions.len() == 2 && actions[1] == Action::Invert)
            );
        } else {
            assert!(editor.undo_log.peek().stacks.is_empty());
            assert!(editor.expanded_stages.peek().is_empty());
        }
    }
}

#[test]
fn disabled_palette_item_keeps_menu_open_without_delivery() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (mut dom, f, _, received) = fixture(true, false);
    click(&dom, "Add first stage");
    dom.render_immediate_to_vec();
    click(&dom, "Map to mouse");
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("aria-expanded=\"true\""));
    assert_eq!(
        received.unwrap().try_recv().unwrap_err(),
        mpsc::TryRecvError::Empty
    );
    assert!(f.editor.borrow().unwrap().undo_log.peek().stacks.is_empty());
}

#[test]
fn rendered_handler_rejects_an_identical_replacement_profile_before_rerender() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (dom, f, _, received) = fixture(false, false);
    let replacement = Profile::new(
        "Fixture".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: Some("Fixture".into()),
            actions: vec![original()],
        }],
        vec![],
        "Default".into(),
    );
    f.state.write().active_profile = Some(replacement);
    f.meta
        .borrow()
        .unwrap()
        .set(inputforge_app::bridge::prepare(&f.state.read(), None).meta);
    click(&dom, "Pulse");
    assert!(
        received.unwrap().try_recv().is_err(),
        "old rendered handler must not edit the replacement profile"
    );
    assert!(f.editor.borrow().unwrap().undo_log.peek().stacks.is_empty());
}

#[test]
fn stage_menu_retains_failed_delete_and_closes_after_success() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for failed in [true, false] {
        let (mut dom, f, _, received) = fixture(false, failed);
        f.menu.set(true);
        let mut editor = f.editor.borrow().unwrap();
        let stage = StageId(vec![StageIdSegment::Index(0)]);
        editor
            .stage_menu
            .set(Some(crate::frame::mapping_editor::StageMenuState {
                stage: stage.clone(),
                x: 1.0,
                y: 1.0,
            }));
        editor.expanded_stages.write().insert(stage);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate_to_vec();
        click(&dom, "Move up");
        dom.render_immediate_to_vec();
        assert!(editor.stage_menu.peek().is_some());
        if let Some(received) = &received {
            assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        }
        click(&dom, "Delete");
        dom.render_immediate_to_vec();
        assert_eq!(editor.stage_menu.peek().is_some(), failed);
        assert_eq!(!editor.expanded_stages.peek().is_empty(), failed);
        if let Some(received) = received {
            assert!(
                matches!(received.try_recv().unwrap(), EngineCommand::SetMapping { actions, .. } if actions.is_empty())
            );
        } else {
            let html = dioxus_ssr::render(&dom);
            assert!(html.contains("role=\"alert\""), "{html}");
            assert!(editor.undo_log.peek().stacks.is_empty());
        }
    }
}
