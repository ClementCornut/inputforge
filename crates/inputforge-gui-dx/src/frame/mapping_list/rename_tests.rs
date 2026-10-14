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
    handler: RefCell<Option<Callback<String>>>,
    config: RefCell<Option<Signal<ConfigSnapshot>>>,
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
    let summary = use_hook(|| config.peek().mappings[0].clone());
    let renaming = use_signal(|| Some(InputAddress::Unbound));
    let handler = Callback::new(move |raw: String| run_commit(&raw, &summary, renaming));
    *f.handler.borrow_mut() = Some(handler);
    *f.config.borrow_mut() = Some(config);
    rsx! { div {} }
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
        f.handler.borrow().unwrap().call("Requested".into());
        if stale {
            assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        } else {
            assert!(
                matches!(rx.try_recv().unwrap(), EngineCommand::SetMapping { name, actions, .. } if name.as_deref() == Some("Requested") && actions == vec![Action::Invert])
            );
        }
    }
}
