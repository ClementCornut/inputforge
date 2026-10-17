//! Capture shares the editor poller and releases listeners on cancellation and teardown.
#[path = "live_capture/support.rs"]
mod support;
use dioxus::prelude::*;
use inputforge_core::{
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::{DeviceId, InputAddress, InputId, InputValue},
};
use inputforge_gui_dx::{
    EditorNavigation, PanelSlot,
    capture::{Capture, CaptureFilter, Target},
    patterns::live_capture::use_capture,
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
    time::Duration,
};

struct Fixture {
    state: Arc<RwLock<AppState>>,
    surface: RefCell<Option<Capture>>,
    received: RefCell<Vec<InputAddress>>,
}
fn address() -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("evdev:v1:polling".into()),
        input: InputId::Button { index: 2 },
    }
}
#[component]
fn CaptureProbe() -> Element {
    let fixture = use_context::<Rc<Fixture>>();
    let target = Target {
        profile: fixture
            .state
            .read()
            .active_profile
            .as_ref()
            .map(|p| p.id().clone()),
        mode: "Default".into(),
        field: "probe".into(),
        ..Default::default()
    };
    let output = Rc::clone(&fixture);
    let capture = use_capture(
        target,
        CaptureFilter::Any,
        move |address| {
            assert!(
                output.state.try_write().is_some(),
                "consumer delivery must release engine guard"
            );
            output.received.borrow_mut().push(address);
        },
        |()| {},
    );
    *fixture.surface.borrow_mut() = Some(capture);
    rsx! {
        if *capture.listening.read() { span { role: "status", "aria-live": "polite", "Listening probe" } }
    }
}
fn editor() -> Element {
    let fixture = use_context::<Rc<Fixture>>();
    let navigation = EditorNavigation {
        panel: use_signal(|| PanelSlot::Devices),
        calibration: use_signal(|| false),
    };
    use_context_provider(|| navigation);
    let (commands, _) = mpsc::channel();
    let capture = use_hook(|| {
        inputforge_app::live_capture::Controller::new(Arc::clone(&fixture.state), commands.clone())
    });
    let state = Arc::clone(&fixture.state);
    let tick_capture = capture.clone();
    let start = Callback::new(move |publisher| {
        inputforge_app::bridge::spawn_polling_task(
            Arc::clone(&state),
            publisher,
            tick_capture.clone(),
        );
    });
    let controllers =
        inputforge_app::controllers::bindings(Arc::clone(&fixture.state), commands.clone());
    let keyboard =
        use_hook(|| inputforge_app::keyboard_capture::Controller::new(Arc::clone(&fixture.state)));
    let profiles = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&fixture.state), commands.clone())
    });
    let settings = use_hook(|| {
        inputforge_app::settings::Controller::new(Arc::clone(&fixture.state), commands.clone())
    });
    let editor = inputforge_gui_dx::render_editor(
        inputforge_gui_dx::routing::Bindings {
            dispatch: Callback::new(|_| {}),
        },
        controllers,
        capture.bindings(),
        keyboard.bindings(),
        use_hook(|| {
            inputforge_app::mapping_edits::Controller::new(
                Arc::clone(&fixture.state),
                commands.clone(),
            )
        })
        .bindings(),
        profiles.bindings(),
        settings.bindings(),
        inputforge_app::notifications::bindings(),
        inputforge_app::ui::bindings(),
        start,
    );
    rsx! { {editor} CaptureProbe {} }
}
fn fixture() -> (VirtualDom, Rc<Fixture>, Rc<support::Document>) {
    let profile = Profile::new(
        "Capture poller".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    let mut state = AppState::with_profile(profile);
    state.session.ready = true;
    state
        .input_cache
        .update(&address(), &InputValue::Button { pressed: false });
    let fixture = Rc::new(Fixture {
        state: Arc::new(RwLock::new(state)),
        surface: RefCell::new(None),
        received: RefCell::new(vec![]),
    });
    let document = Rc::new(support::Document::default());
    let mut dom = VirtualDom::new(editor);
    dom.provide_root_context(Rc::clone(&fixture));
    dom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    (dom, fixture, document)
}
async fn drive(dom: &mut VirtualDom, millis: u64) {
    tokio::time::advance(Duration::from_millis(millis)).await;
    dom.render_immediate_to_vec();
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
}

#[test]
#[expect(
    clippy::await_holding_lock,
    reason = "hold writer through one capture tick to prove nonblocking skips"
)]
fn unchanged_input_completes_on_existing_ticks_and_contention_does_not_duplicate_delivery() {
    runtime().block_on(async {
        let (mut dom, fixture, document) = fixture();
        dom.in_scope(ScopeId::APP, || {
            fixture.surface.borrow().unwrap().start.call(());
        });
        drive(&mut dom, 0).await;
        assert!(dioxus_ssr::render(&dom).contains("Listening probe"));
        fixture
            .state
            .write()
            .input_cache
            .update(&address(), &InputValue::Button { pressed: true });
        drive(&mut dom, 15).await;
        assert_eq!(fixture.received.borrow().len(), 0);
        drive(&mut dom, 1).await;
        drive(&mut dom, 48).await;
        assert!(
            fixture.received.borrow().is_empty(),
            "50 ms debounce must not expire at 48 ms"
        );
        let guard = fixture.state.write();
        drive(&mut dom, 16).await;
        assert_eq!(fixture.received.borrow().len(), 0);
        drop(guard);
        drive(&mut dom, 16).await;
        assert_eq!(*fixture.received.borrow(), vec![address()]);
        drive(&mut dom, 32).await;
        assert_eq!(*fixture.received.borrow(), vec![address()]);
        assert!(!dioxus_ssr::render(&dom).contains("Listening probe"));
        assert_eq!(
            document.listeners.borrow()[0].borrow().sent,
            vec!["__shutdown__"]
        );
        drop(dom);
        assert_eq!(Arc::strong_count(&fixture.state), 1);
    });
}

#[test]
fn stale_escape_cannot_cancel_rearmed_capture_and_waiting_listener_is_removed_on_drop() {
    runtime().block_on(async {
        let (mut dom, fixture, document) = fixture();
        dom.in_scope(ScopeId::APP, || {
            fixture.surface.borrow().unwrap().start.call(());
        });
        drive(&mut dom, 0).await;
        document.listeners.borrow()[0].borrow_mut().escape();
        dom.in_scope(ScopeId::APP, || {
            fixture.surface.borrow().unwrap().start.call(());
        });
        drive(&mut dom, 0).await;
        assert!(dioxus_ssr::render(&dom).contains("Listening probe"));
        assert_eq!(
            document.listeners.borrow()[0].borrow().sent,
            vec!["__shutdown__"]
        );
        document.listeners.borrow()[1].borrow_mut().escape();
        drive(&mut dom, 0).await;
        assert!(!dioxus_ssr::render(&dom).contains("Listening probe"));
        dom.in_scope(ScopeId::APP, || {
            fixture.surface.borrow().unwrap().start.call(());
        });
        drive(&mut dom, 0).await;
        drop(dom);
        assert_eq!(
            document.listeners.borrow()[2].borrow().sent,
            vec!["__shutdown__"]
        );
        assert_eq!(
            document.scripts.borrow().len(),
            6,
            "each installation must dispatch its cleanup, including scope teardown"
        );
        assert_eq!(Arc::strong_count(&fixture.state), 1);
        let mut remount = VirtualDom::new(editor);
        remount.provide_root_context(Rc::clone(&fixture));
        remount.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
        remount.rebuild_in_place();
        remount.in_scope(ScopeId::APP, || {
            fixture.surface.borrow().unwrap().start.call(());
        });
        drive(&mut remount, 0).await;
        assert!(dioxus_ssr::render(&remount).contains("Listening probe"));
        drop(remount);
        assert_eq!(
            document.listeners.borrow()[3].borrow().sent,
            vec!["__shutdown__"]
        );
        assert_eq!(Arc::strong_count(&fixture.state), 1);
    });
}
