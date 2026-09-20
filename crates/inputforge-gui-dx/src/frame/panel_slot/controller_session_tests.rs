use super::controller_session::ControllerSession;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot};
use dioxus::prelude::*;
use inputforge_core::{
    output::traits::ControllerCapabilities,
    state::{AppState, DeviceState, EngineStatus},
    types::{AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, VirtualDeviceConfig},
};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};

#[derive(Clone, Copy, Props, PartialEq)]
struct HarnessProps {
    configurable: bool,
    pending: bool,
    extra_native: bool,
}
fn harness(props: HarnessProps) -> Element {
    let mut state = AppState::new();
    state.session.output_active = true;
    state.session.ready = true;
    state.engine_status = EngineStatus::Stopped;
    state.session.error = Some("fixture permission failure; restore access, then Retry".into());
    state.session.controller_capabilities = ControllerCapabilities {
        configurable: props.configurable,
        min_buttons: 2,
        max_buttons: 53,
        max_hats: 4,
    };
    state.devices.push(DeviceState {
        info: DeviceInfo {
            id: DeviceId("sdl-stick".to_owned()),
            name: "SDL Stick".to_owned(),
            axes: 1,
            buttons: 1,
            hats: 0,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar],
        },
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });
    state.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![],
        button_count: 53,
        hat_count: 1,
    });
    let mut profile = inputforge_core::profile::Profile::new(
        "test".into(),
        vec![],
        inputforge_core::mode::Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    profile
        .set_controllers(inputforge_core::profile::controllers::ControllerConfig {
            virtual_devices: state.virtual_devices.clone(),
            ..Default::default()
        })
        .unwrap();
    if props.extra_native {
        state.virtual_devices.push(VirtualDeviceConfig {
            device_id: 2,
            axes: vec![],
            button_count: 53,
            hat_count: 1,
        });
    }
    state.session.output_layout = state.virtual_devices.clone();
    if props.extra_native {
        state.session.output_layout.truncate(1);
    }
    if props.pending {
        state.session.output_layout[0].button_count = 52;
    }
    state.active_profile = Some(profile);
    let meta = use_signal(|| MetaSnapshot::from_state(&state));
    let config = use_signal(|| ConfigSnapshot::from_state(&state, None));
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    let (commands, _rx) = mpsc::channel();
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

fn render_session(configurable: bool) -> String {
    let mut dom = VirtualDom::new_with_props(
        harness,
        HarnessProps {
            configurable,
            pending: false,
            extra_native: false,
        },
    );
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn stopped_routing_keeps_connected_controllers_and_keeps_shared_controls() {
    let html = render_session(true);
    assert!(!html.contains("Capture for editing"));
    assert!(!html.contains("Rebuild selected bindings"));
    assert!(html.contains("Inputs ready"));
    assert!(html.contains("SDL Stick"));
    assert!(html.contains("Start"));
    assert!(html.contains("Stop"));
    let document = scraper::Html::parse_fragment(&html);
    let fieldsets = scraper::Selector::parse("fieldset").unwrap();
    assert!(
        document
            .select(&fieldsets)
            .all(|field| field.value().attr("disabled").is_none())
    );
    let buttons = scraper::Selector::parse("button").unwrap();
    assert!(
        document
            .select(&buttons)
            .all(|button| button.value().classes().any(|class| class == "if-button"))
    );
    assert!(!html.contains(">Pause<"));
    assert!(!html.contains(">Resume<"));
    assert!(html.contains("fixture permission failure"));
    assert!(html.contains("Add virtual controller"));
}

#[test]
fn fixed_output_layout_is_read_only_with_driver_capabilities() {
    let html = render_session(false);
    assert!(html.contains("provided by the output driver"));
    assert!(html.contains("max=\"53\""));
    assert!(!html.contains("Add virtual controller"));
    assert!(!html.contains("Remove controller"));
    assert!(!html.contains("unavailable on Linux"));
}

#[test]
fn extra_fixed_output_slots_do_not_report_pending_profile_changes() {
    let mut dom = VirtualDom::new_with_props(
        harness,
        HarnessProps {
            configurable: false,
            pending: false,
            extra_native: true,
        },
    );
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Virtual controller 2"));
    assert!(!html.contains("Apply controller changes before starting"));
}

#[derive(Clone, Copy, Props, PartialEq)]
struct AxisHarnessProps {
    profile_loaded: bool,
}

fn axis_harness(props: AxisHarnessProps) -> Element {
    use inputforge_core::{
        profile::controllers::{AxisBinding, ControllerConfig, DeviceBinding},
        settings::AxisSetting,
    };
    let device = DeviceId("sparse-axis-stick".to_owned());
    let mut snapshot = ConfigSnapshot::default();
    let binding = DeviceBinding {
        observed_layout: None,
        device: device.clone(),
        axes: vec![AxisBinding {
            code: 24,
            minimum: -100,
            maximum: 100,
            polarity: AxisPolarity::Bipolar,
        }],
        buttons: vec![],
        hats: vec![],
        unavailable: vec![],
    };
    if props.profile_loaded {
        snapshot.controllers = Some(ControllerConfig {
            bindings: vec![binding.clone()],
            ..ControllerConfig::default()
        });
    }
    snapshot.axis_settings.insert(
        device,
        vec![
            AxisSetting {
                code: 0,
                detected: None,
                override_polarity: Some(AxisPolarity::Bipolar),
            },
            AxisSetting {
                code: 24,
                detected: Some(AxisPolarity::Unipolar),
                override_polarity: Some(AxisPolarity::Unipolar),
            },
        ],
    );
    let mut state = AppState::new();
    if !props.profile_loaded {
        state.session.bindings = vec![binding];
    }
    let meta = use_signal(|| MetaSnapshot::from_state(&state));
    let config = use_signal(|| snapshot);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    let (commands, _) = mpsc::channel();
    use_context_provider(|| AppContext {
        state: Arc::new(RwLock::new(state)),
        commands,
        meta,
        config,
        live,
        settings,
    });
    rsx! { super::axis_settings::AxisSettings {} }
}

#[test]
fn axis_settings_resolve_native_code_and_expose_auto_and_redetection() {
    for profile_loaded in [false, true] {
        let mut dom = VirtualDom::new_with_props(axis_harness, AxisHarnessProps { profile_loaded });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let document = scraper::Html::parse_fragment(&html);
        let select = scraper::Selector::parse("select").unwrap();
        assert_eq!(
            document
                .select(&select)
                .next()
                .unwrap()
                .value()
                .attr("value"),
            Some("minimum")
        );
        assert!(html.contains("if-select "));
        assert!(html.contains("if-button "));
        assert!(html.contains("Detected: rests at minimum"));
        assert!(html.contains(">Auto<"));
        assert!(html.contains("Detect again"));
        assert!(html.contains("Axis 0"));
    }
}

#[test]
fn fixed_layout_profile_change_keeps_apply_enabled_but_capabilities_read_only() {
    let mut dom = VirtualDom::new_with_props(
        harness,
        HarnessProps {
            configurable: false,
            pending: true,
            extra_native: false,
        },
    );
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    let document = scraper::Html::parse_fragment(&html);
    let buttons = scraper::Selector::parse("button").unwrap();
    let apply = document
        .select(&buttons)
        .find(|b| b.text().collect::<String>() == "Apply controller changes")
        .unwrap();
    assert!(apply.value().attr("disabled").is_none());
    assert!(
        !apply
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .any(|parent| parent.value().name() == "fieldset"
                && parent.value().attr("disabled").is_some())
    );
    let inputs = scraper::Selector::parse("input[type=number]").unwrap();
    assert!(
        document
            .select(&inputs)
            .all(|input| input.value().attr("disabled").is_some())
    );
}
