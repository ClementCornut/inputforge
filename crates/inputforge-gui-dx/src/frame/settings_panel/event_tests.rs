//! Real settings handlers retain a rejected prune and report delivery failure.
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot};
use dioxus::prelude::*;
use inputforge_core::{
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    snapshot::{SnapshotId, SnapshotKind},
    state::{ActiveSnapshotRow, AppState},
};
use parking_lot::RwLock;
use std::{
    rc::Rc,
    sync::{Arc, mpsc},
};
fn root() -> Element {
    let mut state = AppState::with_profile(Profile::new(
        "Alpha".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    state.profile_path = Some("/profiles/Alpha.toml".into());
    state.active_snapshot_rows = (0..5)
        .map(|_| ActiveSnapshotRow {
            id: SnapshotId(ulid::Ulid::generate()),
            kind: SnapshotKind::Manual,
            label: None,
            taken_at: chrono::Utc::now(),
            pinned: false,
        })
        .collect();
    let state = Arc::new(RwLock::new(state));
    let frame = inputforge_app::bridge::prepare(&state.read(), None);
    let (commands, rx) = mpsc::channel::<EngineCommand>();
    drop(rx);
    let meta = use_signal(|| frame.meta);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(|| frame.settings);
    use_context_provider(|| AppContext {
        state,
        commands,
        settings,
        meta,
        config,
        live,
    });
    let ts = use_signal(crate::toast::ToastState::default);
    use_context_provider(|| crate::toast::ToastQueue { state: ts });
    rsx! {super::SettingsPanel{}}
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
fn failed_prune_retains_confirmation_and_never_announces_removal() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    event(
        &dom,
        "input",
        2,
        SerializedFormData::new("2".into(), vec![]),
    );
    dom.render_immediate_to_vec();
    event(&dom, "blur", 2, SerializedFocusData {});
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Reduce snapshot buffer to 2?"), "{html}");
    let mut ids = vec![];
    listeners(&dom, dom.base_scope().root_node(), "click", &mut ids);
    event(&dom, "click", ids.len() - 1, SerializedMouseData::default());
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Reduce snapshot buffer to 2?"), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
    assert!(!html.contains("removed from"), "{html}");
}
#[test]
fn invalid_snapshot_text_and_failed_switches_show_feedback_without_false_state() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    event(
        &dom,
        "input",
        2,
        SerializedFormData::new("0".into(), vec![]),
    );
    dom.render_immediate_to_vec();
    event(&dom, "blur", 2, SerializedFocusData {});
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("role=\"alert\""), "{html}");
    assert!(!html.contains("Reduce snapshot buffer"));
    for index in 0..3 {
        event(
            &dom,
            "change",
            index,
            SerializedFormData::new("on".into(), vec![]),
        );
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("role=\"alert\""), "{html}");
        let document = scraper::Html::parse_fragment(&html);
        for (id, checked) in [
            ("if-settings-startup-launch-at-startup", false),
            ("if-settings-startup-start-minimized", false),
            ("if-settings-snapshot-skip-unchanged", true),
        ] {
            let input = document
                .select(&scraper::Selector::parse(&format!("#{id}")).unwrap())
                .next()
                .unwrap();
            assert_eq!(input.value().attr("checked").is_some(), checked, "{html}");
        }
    }
}
#[test]
fn cancel_and_escape_dismiss_prune_without_sending_or_failure_feedback() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    for escape in [false, true] {
        let mut dom = VirtualDom::new(root);
        dom.rebuild_in_place();
        dom.render_immediate_to_vec();
        event(
            &dom,
            "input",
            2,
            SerializedFormData::new("2".into(), vec![]),
        );
        dom.render_immediate_to_vec();
        event(&dom, "blur", 2, SerializedFocusData {});
        dom.render_immediate_to_vec();
        if escape {
            event(&dom, "cancel", 0, SerializedCancelData {});
        } else {
            let mut ids = vec![];
            listeners(&dom, dom.base_scope().root_node(), "click", &mut ids);
            event(&dom, "click", ids.len() - 2, SerializedMouseData::default());
        }
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert!(!html.contains("Reduce snapshot buffer to 2?"), "{html}");
        assert!(!html.contains("role=\"alert\""), "{html}");
    }
}
#[test]
fn invalid_gesture_threshold_keeps_raw_value_and_shows_feedback() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    for index in [0, 1] {
        let mut dom = VirtualDom::new(root);
        dom.rebuild_in_place();
        dom.render_immediate_to_vec();
        event(
            &dom,
            "input",
            index,
            SerializedFormData::new("0".into(), vec![]),
        );
        dom.render_immediate_to_vec();
        event(&dom, "blur", index, SerializedFocusData {});
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("role=\"alert\""), "{html}");
        // SSR renders the committed value attribute, not the browser's raw value property.
        // A second blur must still validate the untouched raw draft after another error.
        event(
            &dom,
            "change",
            0,
            SerializedFormData::new("on".into(), vec![]),
        );
        dom.render_immediate_to_vec();
        assert!(dioxus_ssr::render(&dom).contains("Could not send settings change"));
        event(&dom, "blur", index, SerializedFocusData {});
        dom.render_immediate_to_vec();
        assert!(dioxus_ssr::render(&dom).contains("Enter a whole number between 1 and 10000 ms"));
    }
}
