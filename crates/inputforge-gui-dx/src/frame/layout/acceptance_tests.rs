//! No-profile controls remain reachable through the production layout.
use super::*;
use crate::context::{ConfigSnapshot, LiveSnapshot, SettingsSnapshot};
use inputforge_core::state::AppState;
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

#[derive(Default)]
pub(super) struct Fixture {
    pub(super) panel: RefCell<Option<Signal<crate::PanelSlot>>>,
    pub(super) profile: RefCell<Option<Signal<crate::profiles::View>>>,
    pub(super) state: RefCell<Option<AppState>>,
    pub(super) commands: RefCell<Option<mpsc::Receiver<inputforge_core::engine::EngineCommand>>>,
}
pub(super) fn root() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let state = use_hook(|| Arc::new(RwLock::new(f.state.borrow_mut().take().unwrap_or_default())));
    let commands = use_hook(|| {
        let (commands, receiver) = mpsc::channel();
        *f.commands.borrow_mut() = Some(receiver);
        commands
    });
    crate::test_support::provide_engine(Arc::clone(&state), commands.clone());
    crate::routing::provide_fixture(crate::routing::View::default());
    crate::controllers::provide_fixture(crate::controllers::View::default());
    inputforge_app::notifications::use_provider();
    let profile_controller = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&state), commands.clone())
    });
    let mut profiles = use_hook(|| profile_controller.bindings());
    let mounted_fixture = Rc::clone(&f);
    profiles.mount = Callback::new(move |view| {
        *mounted_fixture.profile.borrow_mut() = Some(view);
        profile_controller.mount(view)
    });
    use_context_provider(|| profiles);
    let controller =
        use_hook(|| inputforge_app::mapping_edits::Controller::new(Arc::clone(&state), commands));
    let bindings = use_hook(|| controller.bindings());
    use_context_provider(|| bindings);
    let meta = use_signal(|| inputforge_app::bridge::prepare(&state.read(), None).meta);
    let ctx = AppContext {
        meta,
        settings: use_signal(SettingsSnapshot::default),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
    };
    use_context_provider(|| ctx);
    let view = crate::frame::use_view_state_provider(meta);
    *f.panel.borrow_mut() = Some(view.panel_slot);
    use_context_provider(|| view);
    rsx! { Layout {} }
}

#[test]
fn profiles_and_settings_are_visible_without_a_loaded_profile() {
    let fixture = Rc::new(Fixture::default());
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&fixture));
    dom.rebuild_in_place();
    for (slot, marker) in [
        (crate::PanelSlot::Profiles, "profile-library"),
        (crate::PanelSlot::Settings, "Settings"),
    ] {
        dom.in_scope(ScopeId::ROOT, || fixture.panel.borrow().unwrap().set(slot));
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("if-empty-state"), "{html}");
        assert!(html.contains("if-panel-slot"), "missing {slot:?}: {html}");
        assert!(html.contains(marker), "{html}");
    }
}

#[derive(Default)]
struct Head(RefCell<Vec<String>>);
impl document::Document for Head {
    fn eval(&self, js: String) -> document::Eval {
        document::Document::eval(&document::NoOpDocument, js)
    }
    fn create_link(&self, props: document::LinkProps) {
        if let Some(href) = props.href {
            self.0.borrow_mut().push(href);
        }
    }
}
fn themed_empty() -> Element {
    rsx! {crate::theme::ThemeProvider { div {"Initial empty workspace"} }}
}
#[test]
fn batch_stylesheet_is_available_before_the_first_batch_visit() {
    let head = Rc::new(Head::default());
    let mut dom = VirtualDom::new(themed_empty);
    dom.provide_root_context(Rc::clone(&head) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    assert!(
        head.0.borrow().iter().any(|href| href.contains("bulk_map")),
        "initial stylesheets: {:?}",
        head.0.borrow()
    );
}

fn click_listeners(dom: &VirtualDom, node: &VNode, ids: &mut Vec<dioxus::core::ElementId>) {
    use dioxus::core::DynamicNode;
    for (i, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs
            .iter()
            .any(|a| a.name == "onclick" || a.name == "click")
            && let Some(id) = node.mounted_dynamic_attribute(i, dom)
        {
            ids.push(id);
        }
    }
    for (i, child) in node.dynamic_nodes.iter().enumerate() {
        match child {
            DynamicNode::Component(c) => {
                if let Some(s) = c.mounted_scope(i, node, dom) {
                    click_listeners(dom, s.root_node(), ids);
                }
            }
            DynamicNode::Fragment(nodes) => {
                for n in nodes {
                    click_listeners(dom, n, ids);
                }
            }
            _ => {}
        }
    }
}
#[test]
fn empty_workspace_new_button_opens_the_real_profile_form() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let fixture = Rc::new(Fixture::default());
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&fixture));
    dom.rebuild_in_place();
    let mut ids = vec![];
    click_listeners(&dom, dom.base_scope().root_node(), &mut ids);
    let id = ids[ids.len() - 2];
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
    dom.render_immediate_to_vec();
    assert_eq!(
        *fixture.panel.borrow().unwrap().peek(),
        crate::PanelSlot::Profiles
    );
    assert_eq!(
        fixture.profile.borrow().unwrap().peek().mode,
        crate::profiles::Mode::New
    );
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Profile name"), "{html}");
}
