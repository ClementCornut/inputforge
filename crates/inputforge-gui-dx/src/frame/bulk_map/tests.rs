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
    AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress, InputId, OutputAddress,
    OutputId, VJoyAxis, VirtualDeviceConfig,
};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::frame::bulk_map::BulkMapPanel;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

pub(super) fn provide(state: AppState) -> (AppContext, mpsc::Receiver<EngineCommand>) {
    let (tx, rx) = mpsc::channel();
    // Mirror production wiring: ctx.config is populated from state via
    // `ConfigSnapshot::from_state`, so the bulk-map panel sees the
    // resolved `device_display_names` map without each test having to
    // build it by hand.
    let initial_config = ConfigSnapshot::from_state(&state, None);
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
        config: use_signal(|| initial_config),
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
        html.contains(r#"aria-pressed="false""#) && html.contains(">Replace</button>"),
        "conflicting row should expose contextual replace control: {html}"
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
fn row_dirty_detection_flags_user_overrides_only() {
    use crate::frame::bulk_map::rows_dirty;

    let baseline = super::derive_rows(&DeviceId("dev-1".to_owned()), 2, 0, 0, &one_vjoy());
    let mut edited = baseline.clone();
    edited[0].target = None;

    assert!(!rows_dirty(&baseline, &baseline));
    assert!(rows_dirty(&edited, &baseline));
}

#[test]
fn row_key_includes_source_device_identity() {
    let dev_1 = super::derive_rows(&DeviceId("dev-1".to_owned()), 1, 0, 0, &one_vjoy());
    let dev_2 = super::derive_rows(&DeviceId("dev-2".to_owned()), 1, 0, 0, &one_vjoy());

    assert_ne!(super::row_key(&dev_1[0]), super::row_key(&dev_2[0]));
}

#[test]
fn source_row_labels_are_user_facing_one_based() {
    use crate::frame::bulk_map::state::RowKind;

    assert_eq!(super::source_row_label(RowKind::Axis, 0), "Axis 1");
    assert_eq!(super::source_row_label(RowKind::Button, 0), "Button 1");
    assert_eq!(super::source_row_label(RowKind::Hat, 0), "Hat 1");
    assert_eq!(super::source_row_label(RowKind::Button, 7), "Button 8");
}

#[test]
fn row_conflicts_detects_any_active_mode_conflict() {
    let map = HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let mut profile = Profile::new(
        "T".to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Default".to_owned(),
    );
    let rows = super::derive_rows(&DeviceId("dev-1".to_owned()), 1, 0, 0, &one_vjoy());
    profile.set_mapping(
        &rows[0].input,
        "Combat",
        Some("Combat throttle".to_owned()),
        vec![Action::Invert],
    );

    let active_modes = vec!["Default".to_owned(), "Combat".to_owned()];

    assert_eq!(
        super::row_conflicts(&rows, &profile, &active_modes),
        vec![true]
    );
}

#[test]
fn row_conflicts_ignores_inactive_mode_conflicts() {
    let map = HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let mut profile = Profile::new(
        "T".to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Default".to_owned(),
    );
    let rows = super::derive_rows(&DeviceId("dev-1".to_owned()), 1, 0, 0, &one_vjoy());
    profile.set_mapping(
        &rows[0].input,
        "Combat",
        Some("Combat throttle".to_owned()),
        vec![Action::Invert],
    );

    assert_eq!(
        super::row_conflicts(&rows, &profile, &["Default".to_owned()]),
        vec![false]
    );
}

#[test]
fn reconcile_missing_source_falls_back_and_rebuilds_rows() {
    use crate::frame::bulk_map::state::WizardState;

    let devices = vec![one_device_state()];
    let vjoys = vec![one_vjoy()];
    let mut wizard = WizardState::empty("Default".to_owned());
    wizard.source_device_id = Some(DeviceId("missing".to_owned()));
    wizard.target_vjoy_id = Some(1);
    wizard.rows = super::derive_rows(&DeviceId("missing".to_owned()), 1, 0, 0, &vjoys[0]);

    super::reconcile_wizard_state(&mut wizard, &devices, &vjoys);

    assert_eq!(wizard.source_device_id, Some(DeviceId("dev-1".to_owned())));
    assert_eq!(wizard.rows.len(), 13);
    assert!(matches!(
        wizard.rows[0].input,
        InputAddress::Bound {
            device: DeviceId(ref id),
            ..
        } if id == "dev-1"
    ));
}

#[test]
fn reconcile_missing_target_falls_back_and_rebuilds_rows() {
    use crate::frame::bulk_map::state::WizardState;

    let devices = vec![one_device_state()];
    let mut fallback = one_vjoy();
    fallback.device_id = 2;
    let vjoys = vec![fallback];
    let mut wizard = WizardState::empty("Default".to_owned());
    wizard.source_device_id = Some(DeviceId("dev-1".to_owned()));
    wizard.target_vjoy_id = Some(9);
    wizard.rows = Vec::new();

    super::reconcile_wizard_state(&mut wizard, &devices, &vjoys);

    assert_eq!(wizard.target_vjoy_id, Some(2));
    assert_eq!(wizard.rows.len(), 13);
    assert!(
        wizard
            .rows
            .iter()
            .all(|row| row.target.as_ref().is_none_or(|target| target.device == 2))
    );
}

#[test]
fn reconcile_changed_capabilities_rebuilds_rows() {
    use crate::frame::bulk_map::state::WizardState;

    let mut device = one_device_state();
    device.info.axes = 2;
    device.info.buttons = 1;
    device.info.hats = 0;
    device.info.axis_polarities = vec![AxisPolarity::Bipolar; 2];
    let devices = vec![device];
    let vjoys = vec![one_vjoy()];
    let mut wizard = WizardState::empty("Default".to_owned());
    wizard.source_device_id = Some(DeviceId("dev-1".to_owned()));
    wizard.target_vjoy_id = Some(1);
    wizard.rows = super::derive_rows(&DeviceId("dev-1".to_owned()), 4, 8, 1, &vjoys[0]);

    super::reconcile_wizard_state(&mut wizard, &devices, &vjoys);

    assert_eq!(wizard.rows.len(), 3);
}

#[test]
fn reconcile_valid_selection_leaves_rows_untouched() {
    use crate::frame::bulk_map::state::WizardState;

    let devices = vec![one_device_state()];
    let vjoys = vec![one_vjoy()];
    let rows = super::derive_rows(&DeviceId("dev-1".to_owned()), 4, 8, 1, &vjoys[0]);
    let mut wizard = WizardState::empty("Default".to_owned());
    wizard.source_device_id = Some(DeviceId("dev-1".to_owned()));
    wizard.target_vjoy_id = Some(1);
    wizard.rows = rows.clone();

    super::reconcile_wizard_state(&mut wizard, &devices, &vjoys);

    assert_eq!(wizard.rows, rows);
}

#[test]
fn reconcile_valid_selection_preserves_row_overrides() {
    use crate::frame::bulk_map::state::WizardState;

    let devices = vec![one_device_state()];
    let vjoys = vec![one_vjoy()];
    let mut rows = super::derive_rows(&DeviceId("dev-1".to_owned()), 4, 8, 1, &vjoys[0]);
    rows[0].replace = true;
    rows[1].target = None;
    let mut wizard = WizardState::empty("Default".to_owned());
    wizard.source_device_id = Some(DeviceId("dev-1".to_owned()));
    wizard.target_vjoy_id = Some(1);
    wizard.rows = rows.clone();

    super::reconcile_wizard_state(&mut wizard, &devices, &vjoys);

    assert_eq!(wizard.rows, rows);
}

#[test]
fn panel_summary_includes_across_n_modes_when_apply_to_all_checked() {
    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
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
fn panel_do_not_map_target_excludes_row_from_apply_count() {
    use crate::frame::bulk_map::{
        apply_for_test,
        state::{RowKind, RowState, WizardState},
    };

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
    let state = AppState::with_profile(profile);
    let row = RowState {
        kind: RowKind::Axis,
        source_index: 0,
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        },
        target: None,
        replace: false,
    };
    let wizard = WizardState::with_seed_rows(vec![row], "Default".to_owned());
    let (tx, rx) = mpsc::channel();

    apply_for_test(&state, &wizard, &["Default".to_owned()], &tx);

    let cmd = rx
        .try_recv()
        .expect("dispatch must always send a command, even if entries empty");
    match cmd {
        EngineCommand::SetMappingsBulk { entries, .. } => {
            assert!(
                entries.is_empty(),
                "do-not-map row must not produce an entry"
            );
        }
        _ => panic!("expected SetMappingsBulk"),
    }
}

#[test]
fn panel_apply_for_test_dispatches_set_mappings_bulk_with_snapshot_label() {
    use crate::frame::bulk_map::{
        apply_for_test,
        state::{RowKind, RowState, WizardState},
    };

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
    let state = AppState::with_profile(profile);
    let row = RowState {
        kind: RowKind::Axis,
        source_index: 0,
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        },
        target: Some(OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }),
        replace: false,
    };
    let mut wizard = WizardState::with_seed_rows(vec![row], "Default".to_owned());
    wizard.source_device_id = Some(DeviceId("dev-1".to_owned()));
    wizard.target_vjoy_id = Some(1);
    let (tx, rx) = mpsc::channel();

    apply_for_test(&state, &wizard, &["Default".to_owned()], &tx);

    let cmd = rx.try_recv().expect("dispatch arrives");
    match cmd {
        EngineCommand::SetMappingsBulk {
            entries,
            snapshot_label,
        } => {
            assert_eq!(entries.len(), 1);
            // The kind badge in the snapshot row already says "Before
            // batch map", so the recovery label carries only the
            // resolved source-device display name and the destination
            // vJoy slot. With no alias / hardware name registered for
            // "dev-1", the resolver falls through to the raw id string
            // (last-resort behaviour of `display_name_for`).
            assert_eq!(snapshot_label, "dev-1 \u{00b7} vJoy 1");
        }
        _ => panic!("expected SetMappingsBulk"),
    }
}

#[test]
fn source_options_use_alias_via_snapshot_accessor() {
    use crate::frame::bulk_map::build_source_options;
    let device = DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "Generic HID Joystick".to_owned(),
            axes: 4,
            buttons: 16,
            hats: 1,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 4],
        },
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    };
    let mut device_display_names = HashMap::new();
    device_display_names.insert(DeviceId("dev-1".to_owned()), "Throttle Quadrant".to_owned());
    let cfg = ConfigSnapshot {
        devices: vec![device.clone()],
        device_display_names,
        ..ConfigSnapshot::default()
    };

    let opts = build_source_options(&[device], &cfg);
    // The label cell shows the alias, never the raw hardware name.
    assert_eq!(opts.len(), 1);
    assert_eq!(opts[0].value, "dev-1");
    assert_eq!(opts[0].label, "Throttle Quadrant");
}

#[test]
fn source_options_fall_back_to_hardware_name_when_no_alias() {
    use crate::frame::bulk_map::build_source_options;
    let device = DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "Generic HID Joystick".to_owned(),
            axes: 4,
            buttons: 16,
            hats: 1,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 4],
        },
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    };
    let mut device_display_names = HashMap::new();
    device_display_names.insert(
        DeviceId("dev-1".to_owned()),
        "Generic HID Joystick".to_owned(),
    );
    let cfg = ConfigSnapshot {
        devices: vec![device.clone()],
        device_display_names,
        ..ConfigSnapshot::default()
    };

    let opts = build_source_options(&[device], &cfg);
    assert_eq!(opts[0].label, "Generic HID Joystick");
}
