//! Layer-5 SSR tests for the bulk-map wizard. Mounts the panel inside
//! a stub-context harness mirroring `frame::mapping_list::tests`.

#![allow(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention"
)]

use std::collections::HashMap;
use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::mode::ModeTree;
use inputforge_core::profile::Profile;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState};
use inputforge_core::types::{
    AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId, VJoyAxis, VirtualDeviceConfig,
};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::frame::bulk_map::BulkMapPanel;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

pub(super) fn provide(state: AppState) -> (AppContext, mpsc::Receiver<EngineCommand>) {
    let (tx, rx) = mpsc::channel();
    let ctx = AppContext {
        state: Arc::new(RwLock::new(state)),
        commands: tx,
        settings: Arc::new(AppSettings::default()),
        meta: use_signal(|| MetaSnapshot {
            profile_name: Some("T".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned()],
            ..MetaSnapshot::default()
        }),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
    };
    use_context_provider(|| ctx.clone());
    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);
    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });
    use_live_capture_provider();
    (ctx, rx)
}

pub(super) fn one_device_state() -> DeviceState {
    DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "FlightStick".to_owned(),
            axes: 4,
            buttons: 8,
            hats: 1,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 4],
        },
        connected: true,
    }
}

pub(super) fn one_vjoy() -> VirtualDeviceConfig {
    VirtualDeviceConfig {
        device_id: 1,
        axes: vec![
            VJoyAxis::X,
            VJoyAxis::Y,
            VJoyAxis::Z,
            VJoyAxis::Rx,
            VJoyAxis::Ry,
            VJoyAxis::Rz,
            VJoyAxis::Slider0,
            VJoyAxis::Slider1,
        ],
        button_count: 32,
        hat_count: 1,
    }
}

pub(super) fn seeded_state(with_vjoy: bool) -> AppState {
    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let profile = Profile::new(
        "T".to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Default".to_owned(),
    );
    let mut state = AppState::with_profile(profile);
    state.devices.push(one_device_state());
    if with_vjoy {
        state.virtual_devices.push(one_vjoy());
    }
    state
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    NoProfile,
    NoVjoy,
    Full,
    WithDisconnected,
}

fn state_for(scenario: Scenario) -> AppState {
    match scenario {
        Scenario::NoProfile => {
            let mut state = AppState::new();
            state.devices.push(one_device_state());
            state.virtual_devices.push(one_vjoy());
            state
        }
        Scenario::NoVjoy => seeded_state(false),
        Scenario::Full => seeded_state(true),
        Scenario::WithDisconnected => {
            let mut state = seeded_state(true);
            state.devices.push(DeviceState {
                info: DeviceInfo {
                    id: DeviceId("dev-2".to_owned()),
                    name: "Unplugged".to_owned(),
                    axes: 0,
                    buttons: 0,
                    hats: 0,
                    instance_path: None,
                    axis_polarities: vec![],
                },
                connected: false,
            });
            state
        }
    }
}

fn render_panel(scenario: Scenario) -> String {
    #[component]
    fn TestComponent(scenario: Scenario) -> Element {
        let _ = provide(state_for(scenario));
        rsx! { BulkMapPanel {} }
    }

    let mut vdom = VirtualDom::new_with_props(TestComponent, TestComponentProps { scenario });
    vdom.rebuild_in_place();
    render(&vdom)
}

#[test]
fn panel_renders_no_profile_empty_state_when_no_profile_loaded() {
    let html = render_panel(Scenario::NoProfile);

    assert!(html.contains("No profile loaded"), "got: {html}");
}

#[test]
fn panel_renders_no_signal_when_virtual_devices_empty() {
    let html = render_panel(Scenario::NoVjoy);

    assert!(html.contains("No vJoy devices configured"), "got: {html}");
}

#[test]
fn panel_metadata_strip_absent_when_no_vjoys() {
    let html = render_panel(Scenario::NoVjoy);

    assert!(
        !html.contains("if-bulk-map__metadata"),
        "metadata strip must not render: {html}"
    );
}

#[test]
fn panel_disables_apply_button_when_virtual_devices_empty() {
    let html = render_panel(Scenario::NoVjoy);

    assert!(
        html.contains("disabled"),
        "Apply must render with disabled attribute: {html}"
    );
}

#[test]
fn panel_source_picker_lists_only_connected_devices() {
    let html = render_panel(Scenario::WithDisconnected);

    assert!(html.contains("FlightStick"), "got: {html}");
    assert!(
        !html.contains("Unplugged"),
        "disconnected devices must be hidden: {html}"
    );
}

#[test]
fn panel_target_picker_renders_capability_summary() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains("vJoy 1: 8 axes, 32 buttons, 1 hat"),
        "got: {html}"
    );
}

#[test]
fn panel_footer_renders_cancel_and_apply_buttons() {
    let html = render_panel(Scenario::Full);

    assert!(html.contains("Cancel"), "got: {html}");
    assert!(html.contains("Apply"), "got: {html}");
}

#[test]
fn panel_axis_row_renders_compact_bipolar_bar() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains("if-bulk-map__live--axis"),
        "axis live cell class: {html}"
    );
}

#[test]
fn panel_button_row_renders_filled_or_stamped_dot() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains("if-bulk-map__live--button"),
        "button live cell class: {html}"
    );
}

#[test]
fn panel_hat_row_renders_cardinal_letter() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains("if-bulk-map__live--hat"),
        "hat live cell class: {html}"
    );
}

#[test]
fn panel_target_picker_options_render_axis_human_labels() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains("X axis"),
        "axis option label must use X axis format: {html}"
    );
    assert!(
        html.contains("Slider 0"),
        "slider option label must use Slider 0 format: {html}"
    );
}

#[test]
fn panel_replace_chip_renders_aria_pressed_false_by_default() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains(r#"aria-pressed="false""#),
        "replace chip default: {html}"
    );
}

#[test]
fn panel_axes_group_shows_replace_all_chip_when_axis_conflict_exists() {
    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mut profile = Profile::new(
            "T".to_owned(),
            vec![],
            modes,
            vec![],
            vec![],
            "Default".to_owned(),
        );
        let collide_input = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        profile.set_mapping(
            &collide_input,
            "Default",
            Some("Throttle".to_owned()),
            vec![Action::Invert],
        );
        let mut state = AppState::with_profile(profile);
        state.devices.push(one_device_state());
        state.virtual_devices.push(one_vjoy());
        let _ = provide(state);
        rsx! { BulkMapPanel {} }
    }

    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);

    assert!(
        html.contains("replace all conflicts"),
        "chip must render on Axes group: {html}"
    );
}

#[test]
fn panel_buttons_group_omits_replace_all_chip_when_no_button_conflict() {
    let html = render_panel(Scenario::Full);
    let buttons_section = html.split("Buttons (").nth(1).unwrap_or("");
    let to_next_group = buttons_section.split("Hats (").next().unwrap_or("");

    assert!(
        !to_next_group.contains("replace all conflicts"),
        "no chip on clean group: {html}"
    );
}

#[test]
fn panel_axes_group_shows_include_all_chip_when_a_row_is_do_not_map() {
    fn TestComponent() -> Element {
        let mut state = seeded_state(true);
        if let Some(vjoy) = state.virtual_devices.first_mut() {
            vjoy.axes = vec![VJoyAxis::X, VJoyAxis::Y, VJoyAxis::Z];
        }
        let _ = provide(state);
        rsx! { BulkMapPanel {} }
    }

    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);

    assert!(
        html.contains("include all"),
        "chip must render when at least one row is unmapped: {html}"
    );
}

#[test]
fn panel_axes_group_shows_exclude_all_chip_when_at_least_one_row_has_target() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains("exclude all"),
        "exclude-all chip must render when rows have targets: {html}"
    );
}
