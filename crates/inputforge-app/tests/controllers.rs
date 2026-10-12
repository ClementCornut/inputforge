//! Controller policy and dispatch regressions through the app boundary.
use inputforge_app::controllers::{dispatch, resolve, view};
use inputforge_core::{
    engine::EngineCommand,
    mode::Modes,
    output::traits::ControllerCapabilities,
    profile::{Profile, controllers::ControllerConfig},
    state::{AppState, DeviceState, EngineStatus},
    types::{DeviceDiagnostics, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig},
};
use inputforge_gui_dx::{controllers::Action, snapshots::DevicePanelRow};
use parking_lot::RwLock;
use std::sync::mpsc;

fn slot(id: u8) -> VirtualDeviceConfig {
    VirtualDeviceConfig {
        device_id: id,
        axes: vec![VJoyAxis::X],
        button_count: 8,
        hat_count: 1,
    }
}
fn state() -> AppState {
    let mut state = AppState::with_profile(Profile::new(
        "controllers".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    state.session.controller_capabilities = ControllerCapabilities {
        configurable: true,
        min_buttons: 2,
        max_buttons: 53,
        max_hats: 4,
    };
    state.devices.push(DeviceState {
        info: DeviceInfo {
            id: DeviceId("stick".into()),
            name: "Stick".into(),
            axes: 0,
            buttons: 1,
            hats: 0,
            instance_path: None,
            axis_polarities: vec![],
        },
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });
    set_config(
        &mut state,
        ControllerConfig {
            virtual_devices: vec![slot(1)],
            ..Default::default()
        },
    );
    state
}
fn config(state: &AppState) -> ControllerConfig {
    state
        .active_profile
        .as_ref()
        .unwrap()
        .controllers()
        .cloned()
        .unwrap_or_default()
}
fn set_config(state: &mut AppState, config: ControllerConfig) {
    state
        .active_profile
        .as_mut()
        .unwrap()
        .set_controllers(config)
        .unwrap();
}
fn rows(state: &AppState) -> Vec<DevicePanelRow> {
    inputforge_app::bridge::prepare(state, None)
        .config
        .device_panel_rows
}
fn command(action: Action, state: &AppState) -> Option<EngineCommand> {
    resolve(state, action).unwrap()
}
fn edited(action: Action, state: &AppState) -> ControllerConfig {
    match command(action, state).unwrap() {
        EngineCommand::SetControllerConfig(config) => config,
        other => panic!("unexpected command {other:?}"),
    }
}
fn pending(state: &mut AppState) {
    state.session.output_active = true;
    state.session.output_layout = vec![slot(2)];
}

#[test]
fn edits_recheck_current_locks_and_do_not_require_ready_inputs() {
    let initial = state();
    assert!(!initial.session.ready);
    assert!(view(&initial, &rows(&initial)).editable);
    for change in 0..5 {
        let mut state = state();
        match change {
            0 => state.engine_status = EngineStatus::Running,
            1 => state.engine_status = EngineStatus::Starting,
            2 => state.session.captured.push(DeviceId("stick".into())),
            3 => state.session.offline = true,
            _ => state.active_profile = None,
        }
        pending(&mut state);
        let view = view(&state, &rows(&state));
        assert!(!view.editable);
        assert!(!view.layout_editable);
        assert!(!view.can_apply);
        assert!(!view.can_add);
        for action in [
            Action::Select {
                device: DeviceId("stick".into()),
                selected: true,
            },
            Action::Add,
            Action::Remove { slot: 1 },
            Action::Buttons {
                slot: 1,
                value: "12".into(),
            },
            Action::Apply,
        ] {
            assert_eq!(command(action, &state), None);
        }
    }
    let mut faulted = initial;
    faulted.engine_status = EngineStatus::Faulted;
    assert!(view(&faulted, &rows(&faulted)).editable);
}

#[test]
fn selection_uses_current_ids_without_duplicates_or_layout_commands() {
    let mut state = state();
    let mut current = config(&state);
    current.selected = vec![DeviceId("remembered".into()), DeviceId("stick".into())];
    set_config(&mut state, current);
    let before = config(&state);
    assert_eq!(
        command(
            Action::Select {
                device: DeviceId("stick".into()),
                selected: true
            },
            &state
        ),
        Some(EngineCommand::SelectControllers(vec![
            DeviceId("remembered".into()),
            DeviceId("stick".into())
        ]))
    );
    assert_eq!(
        command(
            Action::Select {
                device: DeviceId("stick".into()),
                selected: false
            },
            &state
        ),
        Some(EngineCommand::SelectControllers(vec![DeviceId(
            "remembered".into()
        )]))
    );
    assert_eq!(config(&state), before);
    assert_eq!(
        command(
            Action::Select {
                device: DeviceId("unknown".into()),
                selected: true
            },
            &state
        ),
        None
    );
    state.devices[0].connected = false;
    assert!(view(&state, &rows(&state)).selections[0].enabled);
    assert!(
        command(
            Action::Select {
                device: DeviceId("stick".into()),
                selected: false
            },
            &state
        )
        .is_some()
    );
    set_config(&mut state, ControllerConfig::default());
    assert!(!view(&state, &rows(&state)).selections[0].enabled);
    assert_eq!(
        command(
            Action::Select {
                device: DeviceId("stick".into()),
                selected: true
            },
            &state
        ),
        None
    );
}

#[test]
fn layout_edits_preserve_selection_and_other_slots_until_explicit_apply() {
    let mut state = state();
    let mut current = config(&state);
    current.selected.push(DeviceId("stick".into()));
    current.virtual_devices.push(slot(3));
    set_config(&mut state, current.clone());
    let changed = edited(
        Action::Buttons {
            slot: 1,
            value: "53".into(),
        },
        &state,
    );
    assert_eq!(changed.selected, current.selected);
    assert_eq!(changed.virtual_devices[1], slot(3));
    assert_eq!(changed.virtual_devices[0].button_count, 53);
    assert_eq!(
        edited(
            Action::Hats {
                slot: 1,
                value: "0".into()
            },
            &state
        )
        .virtual_devices[0]
            .hat_count,
        0
    );
    assert_eq!(
        edited(
            Action::Axis {
                slot: 1,
                axis: VJoyAxis::X,
                enabled: true
            },
            &state
        )
        .virtual_devices[0]
            .axes,
        [VJoyAxis::X]
    );
    assert_eq!(
        edited(
            Action::Axis {
                slot: 1,
                axis: VJoyAxis::X,
                enabled: false
            },
            &state
        )
        .virtual_devices[0]
            .axes,
        []
    );
    let added = edited(Action::Add, &state);
    assert_eq!(
        added
            .virtual_devices
            .iter()
            .map(|s| s.device_id)
            .collect::<Vec<_>>(),
        [1, 3, 2]
    );
    assert_eq!(added.virtual_devices[2].button_count, 32);
    assert_eq!(added.virtual_devices[2].hat_count, 4);
    let removed = edited(Action::Remove { slot: 1 }, &state);
    assert_eq!(removed.virtual_devices, [slot(3)]);
    set_config(&mut state, removed);
    assert_eq!(edited(Action::Add, &state).virtual_devices[1].device_id, 1);
}

#[test]
fn invalid_values_removed_slots_and_full_capacity_cannot_dispatch_edits() {
    let mut state = state();
    for value in ["", "-1", "256", "junk", "1", "54"] {
        let _ = resolve(
            &state,
            Action::Buttons {
                slot: 1,
                value: value.into(),
            },
        )
        .unwrap_err();
    }
    assert_eq!(
        edited(
            Action::Buttons {
                slot: 1,
                value: "2".into()
            },
            &state
        )
        .virtual_devices[0]
            .button_count,
        2
    );
    assert_eq!(
        edited(
            Action::Hats {
                slot: 1,
                value: "4".into()
            },
            &state
        )
        .virtual_devices[0]
            .hat_count,
        4
    );
    let _ = resolve(
        &state,
        Action::Hats {
            slot: 1,
            value: "5".into(),
        },
    )
    .unwrap_err();
    for action in [
        Action::Remove { slot: 2 },
        Action::Buttons {
            slot: 2,
            value: "8".into(),
        },
        Action::Hats {
            slot: 2,
            value: "1".into(),
        },
        Action::Axis {
            slot: 2,
            axis: VJoyAxis::Y,
            enabled: true,
        },
    ] {
        assert_eq!(command(action, &state), None);
    }
    set_config(
        &mut state,
        ControllerConfig {
            virtual_devices: (1..=16).map(slot).collect(),
            ..Default::default()
        },
    );
    assert!(!view(&state, &rows(&state)).can_add);
    assert_eq!(command(Action::Add, &state), None);
}

#[test]
fn fixed_driver_layout_is_read_only_but_pending_apply_remains_available() {
    let mut state = state();
    state.session.controller_capabilities.configurable = false;
    state.virtual_devices = vec![slot(1), slot(2)];
    state.session.output_active = true;
    state.session.output_layout = vec![slot(1)];
    let prepared = view(&state, &rows(&state));
    assert_eq!(prepared.slots, [slot(1), slot(2)]);
    assert!(!prepared.layout_editable);
    assert!(!prepared.pending);
    assert!(!prepared.can_apply);
    assert_eq!(command(Action::Add, &state), None);
    assert_eq!(
        command(
            Action::Buttons {
                slot: 1,
                value: "12".into()
            },
            &state
        ),
        None
    );
    pending(&mut state);
    assert!(view(&state, &rows(&state)).can_apply);
    assert_eq!(
        command(Action::Apply, &state),
        Some(EngineCommand::ApplyControllerChanges)
    );
}

#[test]
fn apply_rechecks_retained_layout_and_reports_failed_delivery_without_mutating_state() {
    let mut state = state();
    assert_eq!(command(Action::Apply, &state), None);
    pending(&mut state);
    let before = config(&state);
    assert_eq!(
        command(Action::Apply, &state),
        Some(EngineCommand::ApplyControllerChanges)
    );
    assert_eq!(config(&state), before);
    state.session.output_layout = vec![slot(1)];
    assert_eq!(command(Action::Apply, &state), None);
    pending(&mut state);
    let (tx, rx) = mpsc::channel();
    drop(rx);
    let state = RwLock::new(state);
    assert!(dispatch(Action::Apply, &state, &tx).is_err());
    assert!(state.try_write().is_some());
    assert_eq!(config(&state.read()), before);
}

#[test]
fn dispatch_revalidates_a_previously_enabled_control_and_preserves_engine_state() {
    let state = RwLock::new(state());
    let (tx, rx) = mpsc::channel();
    assert!(view(&state.read(), &rows(&state.read())).can_add);
    let before = config(&state.read());
    state.write().engine_status = EngineStatus::Starting;
    dispatch(Action::Add, &state, &tx).unwrap();
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    state.write().engine_status = EngineStatus::Stopped;
    dispatch(Action::Add, &state, &tx).unwrap();
    let EngineCommand::SetControllerConfig(next) = rx.try_recv().unwrap() else {
        panic!("expected configuration command");
    };
    assert_eq!(next.virtual_devices[1].device_id, 2);
    assert_eq!(config(&state.read()), before);
    assert!(state.try_write().is_some());
}

#[test]
fn backend_limits_are_shared_for_edits_apply_and_default_slot_counts() {
    for (min_buttons, max_buttons, max_hats, expected_buttons) in [(40, 53, 4, 40), (2, 8, 0, 8)] {
        let mut state = state();
        state.session.controller_capabilities = ControllerCapabilities {
            configurable: true,
            min_buttons,
            max_buttons,
            max_hats,
        };
        let mut current = config(&state);
        current.virtual_devices[0].button_count = min_buttons;
        current.virtual_devices[0].hat_count = max_hats;
        set_config(&mut state, current);
        let added = edited(Action::Add, &state);
        assert_eq!(added.virtual_devices[1].button_count, expected_buttons);
        assert_eq!(added.virtual_devices[1].hat_count, max_hats);
        pending(&mut state);
        assert_eq!(
            command(Action::Apply, &state),
            Some(EngineCommand::ApplyControllerChanges)
        );
        state.session.controller_capabilities.max_buttons = min_buttons - 1;
        let (tx, rx) = mpsc::channel();
        let state = RwLock::new(state);
        assert!(dispatch(Action::Apply, &state, &tx).is_err());
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        assert!(state.read().session.output_active);
        assert_eq!(state.read().session.output_layout, [slot(2)]);
    }
}
