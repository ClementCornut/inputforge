use super::document::Document;
use dioxus::prelude::*;
use inputforge_app::keyboard_capture::Controller;
use inputforge_core::{
    action::{Action, Mapping, OutputBehavior},
    mode::Modes,
    pipeline::ActionPathSegment,
    profile::Profile,
    state::AppState,
    types::{InputAddress, KeyCombo, PhysicalKey},
};
use inputforge_gui_dx::{
    capture,
    keyboard_capture::{Bindings, Destination, Publication, Request, Target},
};
use parking_lot::RwLock;
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(super) fn profile() -> Profile {
    Profile::new(
        "Same display name".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Other".into()]).unwrap(),
        vec![Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: None,
            actions: vec![Action::MapToKeyboard {
                key: KeyCombo {
                    key: PhysicalKey::F11,
                    modifiers: vec![],
                },
                behavior: OutputBehavior::Hold,
            }],
        }],
        vec![],
        "Default".into(),
    )
}
#[derive(Clone)]
pub(super) struct Fixture {
    pub state: Arc<RwLock<AppState>>,
    pub bindings: Bindings,
    pub mode: Signal<String>,
    pub selection: Signal<Option<inputforge_gui_dx::snapshots::MappingKey>>,
    pub requests: [Request; 2],
    pub results: Rc<RefCell<Vec<(usize, PhysicalKey, bool)>>>,
}
impl Fixture {
    pub(super) fn start(&self, index: usize) {
        self.bindings.start.call(self.requests[index]);
        assert!(
            *self.requests[index].active.peek(),
            "start must publish listening state"
        );
    }
}

pub(super) fn fixture() -> (VirtualDom, Fixture, Rc<Document>) {
    type Output = Rc<RefCell<Option<Fixture>>>;
    fn root() -> Element {
        let state = use_context::<Arc<RwLock<AppState>>>();
        let controller = use_hook(|| Controller::new(Arc::clone(&state)));
        let bindings = controller.bindings();
        let mode = use_signal(|| "Default".into());
        let selection = use_signal(|| Some(("Default".into(), InputAddress::Unbound)));
        use_hook(|| bindings.install.call(Publication { mode, selection }));
        use_drop(move || bindings.dispose.call(()));
        let mapping = state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&InputAddress::Unbound, "Default")
            .unwrap()
            .clone();
        let target = Target {
            mapping: capture::Target {
                profile: state.read().active_profile.as_ref().map(|p| p.id().clone()),
                mode: "Default".into(),
                mapping: Some(InputAddress::Unbound),
                name: None,
                actions: Some(mapping.actions),
                field: "keyboard".into(),
            },
            destination: Destination::Edit(vec![ActionPathSegment::Index(0)]),
        };
        let targets = [use_signal(|| target.clone()), use_signal(|| target.clone())];
        let active = [use_signal(|| false), use_signal(|| false)];
        let pending = [use_signal(|| false), use_signal(|| false)];
        let hint = [use_signal(|| None), use_signal(|| None)];
        let results = use_hook(|| Rc::new(RefCell::new(vec![])));
        let requests = std::array::from_fn(|i| {
            let results = Rc::clone(&results);
            let state = Arc::clone(&state);
            Request {
                owner: ScopeId(i + 10),
                target: targets[i],
                active: active[i],
                pending: pending[i],
                hint: hint[i],
                on_commit: Callback::new(move |combo: KeyCombo| {
                    assert!(
                        state.try_write().is_some(),
                        "completion must release engine guard"
                    );
                    results.borrow_mut().push((i, combo.key, *active[i].peek()));
                }),
            }
        });
        *use_context::<Output>().borrow_mut() = Some(Fixture {
            state,
            bindings,
            mode,
            selection,
            requests,
            results,
        });
        rsx! { div {} }
    }
    let document = Rc::new(Document::default());
    let output = Rc::new(RefCell::new(None));
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Arc::new(RwLock::new(AppState::with_profile(profile()))));
    dom.provide_root_context(Rc::clone(&output));
    dom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    let f = output.borrow_mut().take().unwrap();
    (dom, f, document)
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
