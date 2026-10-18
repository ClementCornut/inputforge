//! Real inspector handlers expose rejected alias writes and keep raw drafts.
use dioxus::prelude::*;
use parking_lot::RwLock;
use std::{
    rc::Rc,
    sync::{Arc, mpsc},
};
fn root() -> Element {
    let mut state = inputforge_core::state::AppState::new();
    let id = inputforge_core::types::DeviceId("old".into());
    let info = inputforge_core::types::DeviceInfo {
        id: id.clone(),
        name: "Old device".into(),
        axes: 0,
        buttons: 1,
        hats: 0,
        instance_path: None,
        axis_polarities: vec![],
    };
    state.device_registry.insert(
        id,
        inputforge_core::settings::DeviceRecord {
            info,
            diagnostics: inputforge_core::types::DeviceDiagnostics::default(),
            axis_settings: vec![],
            last_seen_unix_ms: None,
        },
    );
    let frame = inputforge_app::bridge::prepare(&state, None);
    let state = Arc::new(RwLock::new(state));
    let (commands, rx) = mpsc::channel();
    drop(rx);
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    let live = use_signal(|| frame.live);
    let settings = use_signal(|| frame.settings);
    use_context_provider(|| {
        crate::test_support::provide_engine(state, commands);
        crate::context::AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    rsx! {super::DevicePanel{}}
}
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
fn event<T: 'static>(dom: &VirtualDom, name: &str, index: usize, data: T) {
    let mut ids = vec![];
    listeners(dom, dom.base_scope().root_node(), name, &mut ids);
    dom.runtime().handle_event(
        name,
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
            false,
        ),
        ids[index],
    );
}
#[test]
fn rejected_alias_save_retains_draft_and_accessible_error() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    event(
        &dom,
        "input",
        0,
        SerializedFormData::new(" New alias ".into(), vec![]),
    );
    dom.render_immediate_to_vec();
    event(&dom, "click", 1, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("role=\"alert\""), "{html}");
    assert!(html.contains(" New alias "), "{html}");
}
#[test]
fn rapid_alias_input_does_not_echo_values_and_keeps_failed_exact_draft() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    for raw in ["D", "Dev", "Device été 🕹"] {
        event(
            &dom,
            "input",
            0,
            SerializedFormData::new(raw.into(), vec![]),
        );
    }
    let mutations = dom.render_immediate_to_vec();
    assert!(
        !mutations.edits.iter().any(|edit| matches!(
            edit,
            dioxus::core::Mutation::SetAttribute { name: "value", .. }
        )),
        "alias typing echoed an old native draft: {:?}",
        mutations.edits
    );
    event(&dom, "click", 1, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("Device été 🕹"),
        "failed save lost text: {html}"
    );
    assert!(
        html.contains("role=\"alert\""),
        "failed save hid feedback: {html}"
    );
}
#[test]
fn enter_failure_retains_draft_and_escape_cancels_through_real_handlers() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    event(
        &dom,
        "input",
        0,
        SerializedFormData::new(" draft ".into(), vec![]),
    );
    dom.render_immediate_to_vec();
    for key in [Key::Enter, Key::Escape] {
        let data = SerializedKeyboardData::new(
            key.clone(),
            Code::Enter,
            Location::Standard,
            false,
            Modifiers::empty(),
            false,
        );
        event(&dom, "keydown", 0, data);
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        if key == Key::Enter {
            assert!(html.contains("role=\"alert\""), "{html}");
            assert!(html.contains(" draft "), "{html}");
        } else {
            assert!(!html.contains("role=\"alert\""), "{html}");
            assert!(!html.contains(" draft "), "{html}");
        }
    }
}
