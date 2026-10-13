use dioxus::prelude::*;
use inputforge_app::live_capture::Controller;
use inputforge_core::{
    action::Mapping,
    mode::Modes,
    profile::{Profile, controllers::DeviceBinding},
    state::AppState,
    types::{DeviceId, InputAddress, InputId, InputValue},
};
use inputforge_gui_dx::capture::{Bindings, CaptureFilter, Publication, Request, Target};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

pub(super) fn address(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("evdev:v1:sparse".into()),
        input: InputId::Button { index },
    }
}
pub(super) fn profile() -> Profile {
    Profile::new(
        "Capture fixture".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Other".into()]).unwrap(),
        vec![Mapping {
            input: address(0),
            mode: "Default".into(),
            name: None,
            actions: vec![],
        }],
        vec![],
        "Default".into(),
    )
}

#[derive(Clone)]
pub(super) struct Fixture {
    pub(super) state: Arc<RwLock<AppState>>,
    pub(super) controller: Controller,
    pub(super) bindings: Bindings,
    pub(super) active: Signal<bool>,
    pub(super) mode: Signal<String>,
    pub(super) selection: Signal<Option<inputforge_gui_dx::snapshots::MappingKey>>,
    pub(super) requests: [Request; 2],
    pub(super) results: Rc<RefCell<Vec<InputAddress>>>,
    pub(super) cancelled: Rc<RefCell<Vec<usize>>>,
}
impl Fixture {
    pub(super) fn start(&self, index: usize) {
        self.bindings.start.call(self.requests[index]);
    }
    pub(super) fn sample(&self, millis: u64, now: Instant) {
        self.controller
            .sample(&self.state.read(), now + Duration::from_millis(millis));
    }
    pub(super) fn tick(&self, millis: u64, now: Instant) {
        self.sample(millis, now);
        self.controller.deliver();
    }
    pub(super) fn button(&self, index: u8, pressed: bool) {
        self.state
            .write()
            .input_cache
            .update(&address(index), &InputValue::Button { pressed });
    }
    pub(super) fn pending(&self, now: Instant) {
        self.start(0);
        self.tick(0, now);
        self.button(1, true);
        self.tick(16, now);
    }
}

pub(super) fn fixture() -> (
    VirtualDom,
    Fixture,
    mpsc::Receiver<inputforge_core::engine::EngineCommand>,
) {
    type Output = Rc<RefCell<Option<Fixture>>>;
    fn root() -> Element {
        let state = use_context::<Arc<RwLock<AppState>>>();
        let commands = use_context::<mpsc::Sender<inputforge_core::engine::EngineCommand>>();
        let output = use_context::<Output>();
        let controller = use_hook(|| Controller::new(Arc::clone(&state), commands));
        let bindings = controller.bindings();
        let active = use_signal(|| false);
        let mode = use_signal(|| "Default".into());
        let selection = use_signal(|| Some(("Default".into(), address(0))));
        use_hook(|| {
            bindings.install.call(Publication {
                active,
                mode,
                selection,
            });
        });
        use_drop(move || bindings.dispose.call(()));
        let target = Target {
            profile: state.read().active_profile.as_ref().map(|p| p.id().clone()),
            mode: "Default".into(),
            mapping: Some(address(0)),
            name: None,
            actions: Some(vec![]),
            field: "primary".into(),
        };
        let targets = [use_signal(|| target.clone()), use_signal(|| target.clone())];
        let listening = [use_signal(|| false), use_signal(|| false)];
        let results = use_hook(|| Rc::new(RefCell::new(Vec::new())));
        let cancelled = use_hook(|| Rc::new(RefCell::new(Vec::new())));
        let requests = std::array::from_fn(|index| {
            let results = Rc::clone(&results);
            let cancelled = Rc::clone(&cancelled);
            let state = Arc::clone(&state);
            Request {
                owner: ScopeId(index + 10),
                filter: CaptureFilter::Any,
                target: targets[index],
                listening: listening[index],
                on_capture: Callback::new(move |address| {
                    assert!(
                        state.try_write().is_some(),
                        "capture callbacks must run after releasing the engine guard"
                    );
                    results.borrow_mut().push(address);
                }),
                on_cancel: Callback::new(move |()| cancelled.borrow_mut().push(index)),
            }
        });
        *output.borrow_mut() = Some(Fixture {
            state,
            controller,
            bindings,
            active,
            mode,
            selection,
            requests,
            results,
            cancelled,
        });
        rsx! { div {} }
    }
    let mut state = AppState::with_profile(profile());
    state.session.ready = true;
    state.session.bindings = vec![DeviceBinding {
        device: DeviceId("evdev:v1:sparse".into()),
        observed_layout: None,
        unavailable: vec![],
        axes: vec![],
        buttons: vec![304, 315],
        hats: vec![],
    }];
    for index in 0..2 {
        state
            .input_cache
            .update(&address(index), &InputValue::Button { pressed: false });
    }
    let output = Rc::new(RefCell::new(None));
    let (commands, received) = mpsc::channel();
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Arc::new(RwLock::new(state)));
    dom.provide_root_context(commands);
    dom.provide_root_context(Rc::clone(&output));
    dom.rebuild_in_place();
    let fixture = output.borrow_mut().take().unwrap();
    (dom, fixture, received)
}
