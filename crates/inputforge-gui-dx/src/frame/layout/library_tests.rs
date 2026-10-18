//! Real no-profile navigation must expose library workflows, not an empty-profile placeholder.
use super::acceptance_tests::{Fixture, root};
use dioxus::{
    core::{AttributeValue, DynamicNode, ElementId, TemplateNode},
    prelude::*,
};
use inputforge_core::{
    engine::EngineCommand,
    profile::ProfileId,
    state::{AppState, ProfileLibraryRow, ProfileOrigin},
};
use std::rc::Rc;

fn find(dom: &VirtualDom, node: &VNode, name: &str, value: &str) -> Option<ElementId> {
    for (i, path) in node.template.attr_paths.iter().enumerate() {
        let mut current = &node.template.roots[usize::from(path[0])];
        for offset in &path[1..] {
            if let TemplateNode::Element { children, .. } = current {
                current = &children[usize::from(*offset)];
            }
        }
        if let TemplateNode::Element { tag, attrs, .. } = current {
            let matches_value = |attr_name: &str, attr_value: &str| {
                attr_name == name
                    && (attr_value == value
                        || (name == "class"
                            && attr_value.split_whitespace().any(|token| token == value)))
            };
            let matches = (name == "tag" && *tag == value)
                || attrs.iter().any(|attr| match attr {
                    dioxus::core::TemplateAttribute::Static { name, value, .. } => {
                        matches_value(name, value)
                    }
                    dioxus::core::TemplateAttribute::Dynamic { .. } => false,
                })
                || node.dynamic_attrs[i].iter().any(|attr| match &attr.value {
                    AttributeValue::Text(value) => matches_value(attr.name, value),
                    _ => false,
                });
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
fn click(dom: &VirtualDom, label: &str) {
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
    mouse(dom, id);
}

fn mouse(dom: &VirtualDom, id: ElementId) {
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

#[test]
fn no_profile_navigation_lists_filters_and_opens_existing_library_profiles() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut state = AppState::new();
    #[cfg(unix)]
    let expected_path = {
        use std::os::unix::ffi::OsStringExt as _;
        std::path::PathBuf::from(std::ffi::OsString::from_vec(
            b"/library/Bravo-\xff.toml".to_vec(),
        ))
    };
    #[cfg(not(unix))]
    let expected_path = std::path::PathBuf::from("/library/Bravo.toml");
    for name in ["Alpha", "Bravo"] {
        state.profile_library_rows.push(ProfileLibraryRow {
            name: name.into(),
            path: if name == "Bravo" {
                expected_path.clone()
            } else {
                "/library/Alpha.toml".into()
            },
            profile_id: Some(ProfileId::new()),
            origin: ProfileOrigin::Library,
            is_active: false,
            mode_count: 1,
            last_edited_at: None,
        });
    }
    let fixture = Rc::new(Fixture::default());
    *fixture.state.borrow_mut() = Some(state);
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&fixture));
    dom.rebuild_in_place();
    click(&dom, "Choose profile");
    dom.render_immediate_to_vec();
    assert_eq!(
        *fixture.panel.borrow().unwrap().peek(),
        crate::PanelSlot::Profiles
    );
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Alpha") && html.contains("Bravo"), "{html}");
    assert!(html.contains("Filter profiles"), "{html}");
    assert!(
        html.contains("Open profile from file") && html.contains("New profile"),
        "{html}"
    );
    assert!(html.contains("Load a profile to view snapshots"), "{html}");
    assert!(!html.contains("Snapshot now"), "{html}");

    let id = find(
        &dom,
        dom.base_scope().root_node(),
        "placeholder",
        "Filter profiles",
    )
    .unwrap();
    dom.runtime().handle_event(
        "input",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                "brav".into(),
                vec![],
            )))) as Rc<dyn std::any::Any>,
            true,
        ),
        id,
    );
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(!html.contains("Alpha") && html.contains("Bravo"), "{html}");
    let id = find(
        &dom,
        dom.base_scope().root_node(),
        "class",
        "profile-row__menu-trigger",
    )
    .unwrap();
    mouse(&dom, id);
    dom.render_immediate_to_vec();
    click(&dom, "Open");
    dom.render_immediate_to_vec();
    assert!(
        matches!(fixture.commands.borrow().as_ref().unwrap().try_recv().unwrap(), EngineCommand::LoadProfile(path) if path == expected_path)
    );
    let id = find(
        &dom,
        dom.base_scope().root_node(),
        "class",
        "profile-row--create",
    )
    .unwrap();
    mouse(&dom, id);
    dom.render_immediate_to_vec();
    assert_eq!(
        fixture.profile.borrow().unwrap().peek().mode,
        crate::profiles::Mode::New
    );
    assert!(dioxus_ssr::render(&dom).contains("Profile name"));
}
