//! Real renderer events must deliver app intents, including unparsed numeric values.
use super::controller_session::ControllerSession;
use crate::{
    context::{AppContext, LiveSnapshot, SettingsSnapshot},
    controllers::{Action, AxisRow, Context, Selection, View},
};
use dioxus::{core::Mutation, html::SerializedHtmlEventConverter, prelude::*};
use inputforge_core::{
    mode::Modes,
    profile::{Profile, controllers::ControllerConfig},
    state::AppState,
    types::{DeviceId, VJoyAxis, VirtualDeviceConfig},
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

fn harness() -> Element {
    crate::routing::provide_fixture(crate::routing::View::default());
    let actions = use_context::<Rc<RefCell<Vec<Action>>>>();
    let commands = use_context::<mpsc::Sender<inputforge_core::engine::EngineCommand>>();
    let mut state = AppState::with_profile(Profile::new(
        "fixture".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    state.session.controller_capabilities.configurable = true;
    let slot = VirtualDeviceConfig {
        device_id: 3,
        axes: vec![VJoyAxis::Y],
        button_count: 8,
        hat_count: 1,
    };
    state
        .active_profile
        .as_mut()
        .unwrap()
        .set_controllers(ControllerConfig {
            virtual_devices: vec![slot.clone()],
            ..Default::default()
        })
        .unwrap();
    state.session.output_active = true;
    let frame = inputforge_app::bridge::prepare(&state, None);
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    let view = use_signal(|| View {
        axes: vec![AxisRow {
            device: DeviceId("prepared-axis".into()),
            index: 2,
            code: 24,
            detected: None,
            override_polarity: None,
            can_override: true,
            can_detect: true,
        }],
        editable: true,
        layout_editable: true,
        pending: true,
        can_apply: true,
        can_add: true,
        selections: vec![Selection {
            device: DeviceId("prepared-stick".into()),
            label: "Prepared stick".into(),
            connected: true,
            selected: false,
            enabled: true,
        }],
        capabilities: state.session.controller_capabilities,
        slots: vec![slot],
        ..Default::default()
    });
    let error = use_signal(|| None);
    use_context_provider(|| Context {
        view,
        error,
        dispatch: Callback::new(move |action| actions.borrow_mut().push(action)),
    });
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| AppContext {
        state: Arc::new(RwLock::new(state)),
        commands,
        meta,
        config,
        live,
        settings,
    });
    rsx! { ControllerSession {} }
}

#[test]
fn rendered_controls_forward_app_intents_without_engine_dispatch_or_parsing() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let actions = Rc::new(RefCell::new(Vec::<Action>::new()));
    let (commands, receiver) = mpsc::channel::<inputforge_core::engine::EngineCommand>();
    let mut dom = VirtualDom::new(harness);
    dom.provide_root_context(Rc::clone(&actions));
    dom.provide_root_context(commands);
    let mounted = dom.rebuild_to_vec();
    let element = |label: &str| {
        mounted
            .edits
            .iter()
            .find_map(|edit| match edit {
                Mutation::SetAttribute {
                    name: "aria-label" | "id",
                    value: dioxus::core::AttributeValue::Text(value),
                    id,
                    ..
                } if value == label => Some(*id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing rendered control {label}"))
    };
    for (label, value) in [
        ("Prepared stick", "true"),
        ("Buttons for virtual controller 3", "not-a-number"),
        ("Hats for virtual controller 3", "4"),
        ("X for virtual controller 3", "true"),
        ("axis-polarity-prepared-axis-2-24", "unknown"),
    ] {
        let data = PlatformEventData::new(Box::new(SerializedFormData::new(value.into(), vec![])));
        dom.runtime().handle_event(
            "change",
            Event::new(Rc::new(data) as Rc<dyn std::any::Any>, false),
            element(label),
        );
    }
    for label in [
        "Apply controller changes",
        "Remove controller 3",
        "Add virtual controller",
        "Detect prepared-axis · Z again",
    ] {
        let data = PlatformEventData::new(Box::new(SerializedMouseData::default()));
        dom.runtime().handle_event(
            "click",
            Event::new(Rc::new(data) as Rc<dyn std::any::Any>, false),
            element(label),
        );
    }
    assert_eq!(
        *actions.borrow(),
        [
            Action::Select {
                device: DeviceId("prepared-stick".into()),
                selected: true
            },
            Action::Buttons {
                slot: 3,
                value: "not-a-number".into()
            },
            Action::Hats {
                slot: 3,
                value: "4".into()
            },
            Action::Axis {
                slot: 3,
                axis: VJoyAxis::X,
                enabled: true
            },
            Action::Polarity {
                device: DeviceId("prepared-axis".into()),
                axis: 2,
                code: 24,
                value: "unknown".into()
            },
            Action::Apply,
            Action::Remove { slot: 3 },
            Action::Add,
            Action::Detect {
                device: DeviceId("prepared-axis".into()),
                axis: 2,
                code: 24
            },
        ]
    );
    assert_eq!(receiver.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
}
