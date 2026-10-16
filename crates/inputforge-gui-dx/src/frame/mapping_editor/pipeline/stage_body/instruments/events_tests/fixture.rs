use super::document::Document;
use crate::{
    context::{AppContext, LiveSnapshot, SettingsSnapshot},
    frame::mapping_editor::{EditorState, use_editor_state_provider},
};
use dioxus::prelude::*;
use inputforge_core::{
    action::{Action, Mapping},
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::InputAddress,
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
pub(super) struct Fixture {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub action: Action,
    pub editor: RefCell<Option<EditorState>>,
    pub visible: RefCell<Option<Signal<bool>>>,
    pub config: RefCell<Option<Signal<crate::context::ConfigSnapshot>>>,
}
pub(super) fn key() -> (String, InputAddress) {
    ("Default".into(), InputAddress::Unbound)
}
fn harness() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let frame = inputforge_app::bridge::prepare(&f.state.read(), Some(&key()));
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    *f.config.borrow_mut() = Some(config);
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
    use_hook(|| view.selected_mapping.set(Some(key())));
    use_context_provider(|| view);
    let editor = use_editor_state_provider();
    *f.editor.borrow_mut() = Some(editor);
    let visible = use_signal(|| true);
    *f.visible.borrow_mut() = Some(visible);
    if !*visible.read() {
        return rsx! {div{}};
    }
    let stage_id =
        crate::mapping_edits::StageId(vec![crate::mapping_edits::StageIdSegment::Index(0)]);
    match &f.action {
        Action::ResponseCurve { curve } => {
            rsx! {super::super::super::response_curve::ResponseCurveBody{mapping_key:key(),stage_id,curve:curve.clone(),root_actions:vec![f.action.clone()]}}
        }
        Action::Deadzone { config } => {
            rsx! {super::super::super::deadzone::DeadzoneBody{mapping_key:key(),stage_id,config:config.clone(),root_actions:vec![f.action.clone()]}}
        }
        _ => rsx! {},
    }
}
pub(super) fn fixture(
    action: Action,
) -> (
    VirtualDom,
    Rc<Fixture>,
    Rc<Document>,
    mpsc::Receiver<EngineCommand>,
) {
    let profile = Profile::new(
        "Fixture".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![Mapping {
            input: key().1,
            mode: key().0,
            name: Some("Keep".into()),
            actions: vec![action.clone()],
        }],
        vec![],
        "Default".into(),
    );
    let (commands, rx) = mpsc::channel();
    let f = Rc::new(Fixture {
        state: Arc::new(RwLock::new(AppState::with_profile(profile))),
        commands,
        action,
        editor: RefCell::default(),
        visible: RefCell::default(),
        config: RefCell::default(),
    });
    let doc = Rc::new(Document::default());
    let mut dom = VirtualDom::new(harness);
    dom.provide_root_context(Rc::clone(&f));
    dom.provide_root_context(Rc::clone(&doc) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    (dom, f, doc, rx)
}
pub(super) async fn settle(dom: &mut VirtualDom) {
    for _ in 0..4 {
        tokio::task::yield_now().await;
        dom.render_immediate_to_vec();
    }
}
