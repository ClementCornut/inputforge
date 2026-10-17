//! App-rejected operations must remain visible without persisting invalid configuration.
use dioxus::{
    core::{AttributeValue, ElementId, Mutation, Mutations},
    html::SerializedHtmlEventConverter,
    prelude::*,
};
use inputforge_core::{
    engine::EngineCommand,
    mode::Modes,
    output::traits::ControllerCapabilities,
    profile::{Profile, controllers::ControllerConfig},
    state::AppState,
    types::VirtualDeviceConfig,
};
use inputforge_gui_dx::{EditorNavigation, PanelSlot, render_editor, routing};
use parking_lot::RwLock;
use std::{
    any::Any,
    rc::Rc,
    sync::{Arc, mpsc},
};

fn editor() -> Element {
    let state = use_context::<Arc<RwLock<AppState>>>();
    let commands = use_context::<mpsc::Sender<EngineCommand>>();
    let navigation = EditorNavigation {
        panel: use_signal(|| PanelSlot::Devices),
        calibration: use_signal(|| false),
    };
    use_context_provider(|| navigation);
    let capture = use_hook(|| {
        inputforge_app::live_capture::Controller::new(Arc::clone(&state), commands.clone())
    });
    let controllers = inputforge_app::controllers::bindings(Arc::clone(&state), commands.clone());
    let frame = inputforge_app::bridge::prepare(&state.read(), None);
    let keyboard =
        use_hook(|| inputforge_app::keyboard_capture::Controller::new(Arc::clone(&state)));
    let profiles = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&state), commands.clone())
    });
    let settings = use_hook(|| {
        inputforge_app::settings::Controller::new(Arc::clone(&state), commands.clone())
    });
    render_editor(
        routing::Bindings {
            dispatch: Callback::new(|_| {}),
        },
        controllers,
        capture.bindings(),
        keyboard.bindings(),
        use_hook(|| {
            inputforge_app::mapping_edits::Controller::new(Arc::clone(&state), (commands).clone())
        })
        .bindings(),
        profiles.bindings(),
        settings.bindings(),
        inputforge_app::notifications::bindings(),
        inputforge_app::ui::bindings(),
        Callback::new(move |publisher: inputforge_gui_dx::bridge::Publisher| {
            publisher.publish(frame.clone());
        }),
    )
}
fn state(pending: bool) -> Arc<RwLock<AppState>> {
    let mut state = AppState::with_profile(Profile::new(
        "feedback".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    state.session.controller_capabilities = ControllerCapabilities {
        configurable: true,
        min_buttons: 2,
        max_buttons: if pending { 7 } else { 53 },
        max_hats: 4,
    };
    state
        .active_profile
        .as_mut()
        .unwrap()
        .set_controllers(ControllerConfig {
            virtual_devices: vec![VirtualDeviceConfig {
                device_id: 1,
                axes: vec![],
                button_count: 8,
                hat_count: 1,
            }],
            ..Default::default()
        })
        .unwrap();
    state.session.output_active = pending;
    state.session.bindings = vec![inputforge_core::profile::controllers::DeviceBinding {
        device: inputforge_core::types::DeviceId("axis-stick".into()),
        observed_layout: None,
        unavailable: vec![],
        buttons: vec![],
        hats: vec![],
        axes: vec![inputforge_core::profile::controllers::AxisBinding {
            code: 24,
            minimum: -100,
            maximum: 100,
            polarity: inputforge_core::types::AxisPolarity::Bipolar,
        }],
    }];
    state.session.monitored = vec![inputforge_core::types::DeviceId("axis-stick".into())];
    Arc::new(RwLock::new(state))
}
fn mount(state: &Arc<RwLock<AppState>>) -> (VirtualDom, Mutations, mpsc::Receiver<EngineCommand>) {
    let (tx, rx) = mpsc::channel();
    let mut dom = VirtualDom::new(editor);
    dom.provide_root_context(Arc::clone(state));
    dom.provide_root_context(tx);
    let mounted = dom.rebuild_to_vec();
    (dom, mounted, rx)
}
fn element(mounted: &Mutations, label: &str) -> ElementId {
    mounted
        .edits
        .iter()
        .find_map(|edit| match edit {
            Mutation::SetAttribute {
                name: "aria-label" | "id",
                value: AttributeValue::Text(value),
                id,
                ..
            } if value == label => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing control {label}"))
}
fn form(dom: &mut VirtualDom, id: ElementId, value: &str) {
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
        value.into(),
        vec![],
    ))));
    dom.runtime()
        .handle_event("change", Event::new(data, false), id);
    dom.render_immediate_to_vec();
}
fn config(state: &Arc<RwLock<AppState>>) -> ControllerConfig {
    state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .controllers()
        .unwrap()
        .clone()
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_numeric_edit_displays_an_alert_and_valid_delivery_clears_it() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let state = state(false);
    let before = config(&state);
    let (mut dom, mounted, rx) = mount(&state);
    let hats = element(&mounted, "Hats for virtual controller 1");
    form(&mut dom, hats, "5");
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("role=\"alert\""),
        "rejected edits need visible feedback: {html}"
    );
    assert!(html.contains("invalid controller virtual slot"));
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    assert_eq!(config(&state), before);
    form(&mut dom, hats, "2");
    assert!(!dioxus_ssr::render(&dom).contains("role=\"alert\""));
    let EngineCommand::SetControllerConfig(next) = rx.try_recv().unwrap() else {
        panic!("expected valid edit");
    };
    assert_eq!(next.virtual_devices[0].hat_count, 2);
    assert_eq!(
        config(&state),
        before,
        "delivery must not optimistically mutate engine state"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_apply_displays_an_alert_without_touching_retained_outputs() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let state = state(true);
    let before = config(&state);
    let (mut dom, mounted, rx) = mount(&state);
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(
        SerializedMouseData::default(),
    )));
    dom.runtime().handle_event(
        "click",
        Event::new(data, false),
        element(&mounted, "Apply controller changes"),
    );
    dom.render_immediate_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("role=\"alert\""),
        "rejected Apply needs visible feedback: {html}"
    );
    assert!(html.contains("exceeds backend capabilities"));
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    assert_eq!(config(&state), before);
    assert!(state.read().session.output_active);
}

#[tokio::test(flavor = "current_thread")]
async fn failed_delivery_displays_an_alert_without_optimistic_configuration() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let state = state(false);
    let before = config(&state);
    let (mut dom, mounted, rx) = mount(&state);
    drop(rx);
    form(
        &mut dom,
        element(&mounted, "Hats for virtual controller 1"),
        "2",
    );
    assert!(dioxus_ssr::render(&dom).contains("role=\"alert\""));
    assert_eq!(config(&state), before);
}

#[tokio::test(flavor = "current_thread")]
async fn axis_validation_feedback_clears_after_valid_delivery_without_optimistic_settings() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let state = state(false);
    let before = state.read().device_registry.clone();
    let (mut dom, mounted, rx) = mount(&state);
    let axis = element(&mounted, "axis-polarity-axis-stick-0-24");
    form(&mut dom, axis, "invalid");
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("role=\"alert\""),
        "invalid axis choice needs visible feedback: {html}"
    );
    assert!(html.contains("axis interpretation"));
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    form(&mut dom, axis, "minimum");
    assert!(!dioxus_ssr::render(&dom).contains("role=\"alert\""));
    assert_eq!(
        rx.try_recv().unwrap(),
        EngineCommand::SetAxisPolarity {
            device: inputforge_core::types::DeviceId("axis-stick".into()),
            axis: 0,
            polarity: Some(inputforge_core::types::AxisPolarity::Unipolar),
        }
    );
    assert_eq!(state.read().device_registry, before);
}

#[tokio::test(flavor = "current_thread")]
async fn axis_failed_delivery_displays_feedback_without_optimistic_settings() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let state = state(false);
    let before = state.read().device_registry.clone();
    let (mut dom, mounted, rx) = mount(&state);
    drop(rx);
    form(
        &mut dom,
        element(&mounted, "axis-polarity-axis-stick-0-24"),
        "centered",
    );
    assert!(dioxus_ssr::render(&dom).contains("role=\"alert\""));
    assert_eq!(state.read().device_registry, before);
}
