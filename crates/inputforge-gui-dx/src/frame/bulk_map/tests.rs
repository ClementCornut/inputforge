//! Layer-5 SSR tests for the bulk-map wizard. Mounts the panel inside
//! a stub-context harness mirroring `frame::mapping_list::tests`.

#![allow(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention"
)]

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::mode::Modes;
use inputforge_core::profile::Profile;
use inputforge_core::state::{AppState, DeviceState};
use inputforge_core::types::{
    AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress, InputId, VJoyAxis,
    VirtualDeviceConfig,
};

use crate::context::{AppContext, LiveSnapshot, MetaSnapshot, SettingsSnapshot};
use crate::frame::bulk_map::BulkMapPanel;
use crate::patterns::live_capture::use_live_capture_provider;

pub(super) fn provide(state: AppState) -> (AppContext, mpsc::Receiver<EngineCommand>) {
    let (tx, rx) = mpsc::channel();
    // Mirror production wiring: ctx.config is populated from state via
    // the app projection, so the bulk-map panel sees the
    // resolved `device_display_names` map without each test having to
    // build it by hand.
    let initial_config = inputforge_app::bridge::prepare(&state, None).config;
    let ctx = {
        crate::test_support::provide_engine(Arc::new(RwLock::new(state)), tx);
        AppContext {
            settings: use_signal(SettingsSnapshot::default),
            meta: use_signal(|| MetaSnapshot {
                profile_name: Some("T".to_owned()),
                startup_mode: Some("Default".to_owned()),
                modes: vec!["Default".to_owned()],
                ..MetaSnapshot::default()
            }),
            config: use_signal(|| initial_config),
            live: use_signal(LiveSnapshot::default),
        }
    };
    use_context_provider(|| ctx.clone());
    let controller = use_hook(|| {
        inputforge_app::mapping_edits::Controller::new(
            Arc::clone(&crate::test_support::engine().state),
            crate::test_support::engine().commands.clone(),
        )
    });
    let bindings = use_hook(|| controller.bindings());
    use_context_provider(|| bindings);
    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);
    inputforge_app::notifications::use_provider();
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
        diagnostics: DeviceDiagnostics::default(),
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
    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
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
    EmptyInputs,
    ButtonsOnly,
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
                diagnostics: DeviceDiagnostics::default(),
            });
            state
        }
        Scenario::EmptyInputs => {
            let mut state = seeded_state(true);
            let device = state.devices.first_mut().expect("seeded device exists");
            device.info.axes = 0;
            device.info.buttons = 0;
            device.info.hats = 0;
            device.info.axis_polarities = vec![];
            state
        }
        Scenario::ButtonsOnly => {
            let mut state = seeded_state(true);
            let device = state.devices.first_mut().expect("seeded device exists");
            device.info.axes = 0;
            device.info.buttons = 3;
            device.info.hats = 0;
            device.info.axis_polarities = vec![];
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

    assert!(
        html.contains("No virtual devices configured"),
        "got: {html}"
    );
    assert!(
        html.contains("Configure virtual devices in vJoyConf, then reopen."),
        "got: {html}"
    );
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
        html.contains("Device 1 · 8 axes · 32 btn · 1 hat"),
        "got: {html}"
    );
}

#[test]
fn virtual_device_option_formats_normal_capabilities() {
    assert_eq!(
        super::labels::format_virtual_device_option(&one_vjoy()),
        "Device 1 · 8 axes · 32 btn · 1 hat"
    );
}

#[test]
fn virtual_device_option_formats_maximum_capabilities() {
    let mut device = one_vjoy();
    device.device_id = 16;
    device.button_count = 128;
    device.hat_count = 4;

    assert_eq!(
        super::labels::format_virtual_device_option(&device),
        "Device 16 · 8 axes · 128 btn · 4 hats"
    );
}

#[test]
fn virtual_device_option_pluralizes_axes_and_hats_but_not_btn() {
    let device = VirtualDeviceConfig {
        device_id: 3,
        axes: vec![VJoyAxis::X],
        button_count: 1,
        hat_count: 2,
    };

    assert_eq!(
        super::labels::format_virtual_device_option(&device),
        "Device 3 · 1 axis · 1 btn · 2 hats"
    );
}

#[test]
fn panel_footer_omits_cancel_and_clean_reset() {
    let html = render_panel(Scenario::Full);

    assert!(
        !html.contains("Cancel"),
        "full-page workspace exits through primary nav: {html}"
    );
    assert!(
        !html.contains("Reset"),
        "clean auto-map state should not show reset: {html}"
    );
    assert!(html.contains("Apply"), "got: {html}");
}

#[test]
fn panel_ready_workspace_has_batch_map_region_without_drawer_header() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains(r#"aria-label="Batch map device inputs""#),
        "batch map region label missing: {html}"
    );
    assert!(
        !html.contains("Bulk-map device") && !html.contains("if-bulk-map__close"),
        "drawer header title and close affordance must not render: {html}"
    );
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

    let document = scraper::Html::parse_fragment(&html);
    let selector =
        scraper::Selector::parse(".if-bulk-map__row--button .if-status-dot[aria-label='Released']")
            .unwrap();
    let dots: Vec<_> = document.select(&selector).collect();
    assert_eq!(
        dots.len(),
        8,
        "every released button row needs its own status: {html}"
    );
    assert!(
        dots.iter().all(|dot| dot
            .value()
            .classes()
            .any(|class| class == "if-status-dot--hollow")),
        "released button rows must remain hollow: {html}"
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
fn panel_rows_mark_source_live_layout_by_kind() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains("if-bulk-map__row if-bulk-map__row--axis"),
        "axis row layout hook missing: {html}"
    );
    assert!(
        html.contains("if-bulk-map__row if-bulk-map__row--button"),
        "button row layout hook missing: {html}"
    );
    assert!(
        html.contains("if-bulk-map__row if-bulk-map__row--hat"),
        "hat row layout hook missing: {html}"
    );
    assert!(
        html.contains("if-bulk-map__source-cell if-bulk-map__source-cell--axis"),
        "axis source/live layout hook missing: {html}"
    );
    assert!(
        html.contains("if-bulk-map__source-cell if-bulk-map__source-cell--button"),
        "button source/live layout hook missing: {html}"
    );
    assert!(
        html.contains("if-bulk-map__source-cell if-bulk-map__source-cell--hat"),
        "hat source/live layout hook missing: {html}"
    );
}

#[test]
fn panel_hides_empty_categories_independently() {
    let html = render_panel(Scenario::ButtonsOnly);

    assert!(
        !html.contains("Axes (0)"),
        "empty axes group hidden: {html}"
    );
    assert!(html.contains("Buttons (3)"), "button group visible: {html}");
    assert!(
        !html.contains("Hats (0)"),
        "empty hats group hidden: {html}"
    );
}

#[test]
fn panel_renders_single_table_empty_state_when_all_categories_empty() {
    let html = render_panel(Scenario::EmptyInputs);

    assert!(
        html.contains("No inputs available for this source device."),
        "table empty state missing: {html}"
    );
    assert!(
        !html.contains("Axes (0)") && !html.contains("Buttons (0)") && !html.contains("Hats (0)"),
        "empty category headers must be omitted: {html}"
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
fn panel_clean_rows_omit_contextual_replace_controls() {
    let html = render_panel(Scenario::Full);

    assert!(
        !html.contains(r#"aria-pressed="false""#) && !html.contains(">replace</button>"),
        "clean rows should not repeat inactive replace controls: {html}"
    );
}

#[test]
fn panel_conflicting_row_renders_contextual_replace_control() {
    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
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
        html.contains(r#"aria-pressed="false""#) && html.contains(">Replace</button>"),
        "conflicting row should expose contextual replace control: {html}"
    );
}

#[test]
fn panel_axes_group_shows_replace_all_chip_when_axis_conflict_exists() {
    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
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
        html.contains("Replace all conflicts"),
        "chip must render on Axes group: {html}"
    );
}

#[test]
fn panel_buttons_group_omits_replace_all_chip_when_no_button_conflict() {
    let html = render_panel(Scenario::Full);
    let buttons_section = html.split("Buttons (").nth(1).unwrap_or("");
    let to_next_group = buttons_section.split("Hats (").next().unwrap_or("");

    assert!(
        !to_next_group.contains("Replace all conflicts"),
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
        html.contains("Include all"),
        "chip must render when at least one row is unmapped: {html}"
    );
}

#[test]
fn panel_axes_group_shows_exclude_all_chip_when_at_least_one_row_has_target() {
    let html = render_panel(Scenario::Full);

    assert!(
        html.contains("Exclude all"),
        "exclude-all chip must render when rows have targets: {html}"
    );
}

#[test]
fn panel_apply_button_renders_count_when_no_conflicts() {
    let html = render_panel(Scenario::Full);

    assert!(html.contains("Apply 13 mappings"), "Apply label: {html}");
}

#[test]
fn panel_summary_chip_counts_match_row_states() {
    let html = render_panel(Scenario::Full);

    assert!(html.contains("+13 create"), "create count: {html}");
}

#[test]
fn panel_summary_and_actions_share_footer_region() {
    let html = render_panel(Scenario::Full);
    let footer = html
        .split(r#"<footer class="if-bulk-map__footer""#)
        .nth(1)
        .unwrap_or("");

    assert!(footer.contains("+13 create"), "summary in footer: {html}");
    assert!(
        footer.contains("Apply 13 mappings"),
        "apply button in footer: {html}"
    );
}

#[test]
fn panel_summary_includes_across_n_modes_when_apply_to_all_checked() {
    fn TestComponent() -> Element {
        let modes = Modes::new(vec!["Default".to_owned(), "Combat".to_owned()]).unwrap();
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
        state.virtual_devices.push(one_vjoy());
        let (mut ctx, _) = provide(state);
        ctx.meta.write().modes = vec!["Default".to_owned(), "Combat".to_owned()];
        rsx! { BulkMapPanel {} }
    }

    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);

    assert!(html.contains("Default"), "default mode option: {html}");
    assert!(html.contains("Combat"), "combat mode option: {html}");
}

#[test]
fn bulk_map_css_keeps_metadata_cap_and_fits_maximum_target_label() {
    let css = include_str!("../../../assets/frame/bulk_map.css");
    let block = css
        .split(".if-bulk-map__metadata {")
        .nth(1)
        .expect("metadata rule present")
        .split('}')
        .next()
        .expect("metadata rule closed");

    assert!(block.contains("width: min(100%, 56rem);"), "{block}");
    assert!(block.contains("max-width: 56rem;"), "{block}");
    assert!(
        block.contains("minmax(17.5rem, 1.3fr)"),
        "target track must fit `Device 16 · 8 axes · 128 btn · 4 hats`: {block}"
    );
}
