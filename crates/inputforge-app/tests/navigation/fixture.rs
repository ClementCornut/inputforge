use dioxus::prelude::*;
use inputforge_app::mapping_edits::Controller;
use inputforge_core::{
    action::{Action, Mapping},
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::InputAddress,
};
use inputforge_gui_dx::{
    PanelSlot,
    mapping_edits::{Bindings, Publication as History, View},
    navigation::{EditorPublication, Publication, Surface},
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
type Output = Rc<RefCell<Option<(Bindings, Publication, EditorPublication)>>>;
pub(super) fn profile() -> Profile {
    Profile::new(
        "Same".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Other".into()]).unwrap(),
        vec![Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: None,
            actions: vec![Action::Invert],
        }],
        vec![],
        "Default".into(),
    )
}
fn root() -> Element {
    let state = use_context::<Arc<RwLock<AppState>>>();
    let commands = use_context::<mpsc::Sender<EngineCommand>>();
    let controller = use_hook(|| Controller::new(state, commands));
    let bindings = use_hook(|| controller.bindings());
    let nav = Publication {
        main_surface: use_signal(Surface::default),
        editing_mode: use_signal(|| "Default".into()),
        panel_slot: use_signal(PanelSlot::default),
        via_calibration: use_signal(|| false),
        selected_mapping: use_signal(|| None),
    };
    let history = use_signal(View::default);
    let editor = EditorPublication {
        expanded: use_signal(Default::default),
        menu: use_signal(|| None),
        hints: use_signal(Default::default),
        tags: use_signal(Default::default),
        error: use_signal(|| None),
        focus: Callback::new(|_| {}),
    };
    use_hook(|| {
        bindings.navigation.install_editor.call(editor);
        bindings.install.call(History {
            history,
            mode: nav.editing_mode,
            selection: nav.selected_mapping,
        });
        bindings.navigation.install.call(nav);
    });
    *use_context::<Output>().borrow_mut() = Some((bindings, nav, editor));
    use_drop(move || bindings.dispose.call(()));
    rsx! {div{}}
}
pub(super) struct Fixture {
    pub dom: VirtualDom,
    pub bindings: Bindings,
    pub nav: Publication,
    pub editor: EditorPublication,
    pub state: Arc<RwLock<AppState>>,
    pub rx: mpsc::Receiver<EngineCommand>,
}
pub(super) fn mount() -> Fixture {
    let state = Arc::new(RwLock::new(AppState::with_profile(profile())));
    let output: Output = Rc::default();
    let (commands, rx) = mpsc::channel();
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(commands);
    dom.provide_root_context(Arc::clone(&state));
    dom.provide_root_context(Rc::clone(&output));
    dom.rebuild_in_place();
    let (bindings, nav, editor) = output.borrow_mut().take().unwrap();
    Fixture {
        dom,
        bindings,
        nav,
        editor,
        state,
        rx,
    }
}
pub(super) fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
pub(super) async fn drive(dom: &mut VirtualDom) {
    for _ in 0..4 {
        tokio::task::yield_now().await;
        dom.render_immediate_to_vec();
    }
}
