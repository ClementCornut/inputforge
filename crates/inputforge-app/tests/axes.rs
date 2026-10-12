//! Global axis settings must preserve native identity and current sampling availability.
use inputforge_app::{
    bridge::prepare,
    controllers::{dispatch, resolve},
};
use inputforge_core::{
    engine::EngineCommand,
    mode::Modes,
    profile::{
        Profile,
        controllers::{AxisBinding, ControllerConfig, DeviceBinding},
    },
    settings::{AxisSetting, DeviceRecord},
    state::{AppState, EngineStatus},
    types::{AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, InputId},
};
use inputforge_gui_dx::controllers::Action;
use parking_lot::RwLock;
use std::sync::mpsc;

fn device() -> DeviceId {
    DeviceId("sparse-stick".into())
}
fn binding(codes: &[u16]) -> DeviceBinding {
    DeviceBinding {
        device: device(),
        observed_layout: None,
        unavailable: vec![],
        axes: codes
            .iter()
            .map(|code| AxisBinding {
                code: *code,
                minimum: -100,
                maximum: 100,
                polarity: AxisPolarity::Bipolar,
            })
            .collect(),
        buttons: vec![],
        hats: vec![],
    }
}
fn state() -> AppState {
    let mut state = AppState::new();
    state.session.bindings = vec![binding(&[24, 3])];
    state.session.monitored = vec![device()];
    state.device_registry.insert(
        device(),
        DeviceRecord {
            info: DeviceInfo {
                id: device(),
                name: "Stick".into(),
                axes: 2,
                buttons: 0,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![],
            },
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: None,
            axis_settings: vec![AxisSetting {
                code: 24,
                detected: Some(AxisPolarity::Unipolar),
                override_polarity: Some(AxisPolarity::Bipolar),
            }],
        },
    );
    state
}
fn polarity(value: &str) -> Action {
    Action::Polarity {
        device: device(),
        axis: 0,
        code: 24,
        value: value.into(),
    }
}
fn detect() -> Action {
    Action::Detect {
        device: device(),
        axis: 0,
        code: 24,
    }
}
fn saved(state: &mut AppState, table: DeviceBinding) {
    let mut profile = Profile::new(
        "fixture".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    profile
        .set_controllers(ControllerConfig {
            bindings: vec![table],
            ..Default::default()
        })
        .unwrap();
    state.active_profile = Some(profile);
}

#[test]
fn projection_matches_settings_by_native_code_with_live_precedence_and_saved_fallback() {
    let mut state = state();
    saved(&mut state, binding(&[3, 24]));
    let rows = prepare(&state, None).controllers.axes;
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].index, rows[0].code), (0, 24));
    assert_eq!(rows[0].detected, Some(AxisPolarity::Unipolar));
    assert_eq!(rows[0].override_polarity, Some(AxisPolarity::Bipolar));
    assert!(rows[0].can_detect);
    assert_eq!(rows[1].override_polarity, None);
    state.session.bindings.clear();
    state.session.monitored.clear();
    let rows = prepare(&state, None).controllers.axes;
    assert_eq!((rows[1].index, rows[1].code), (1, 24));
    assert_eq!(rows[1].detected, Some(AxisPolarity::Unipolar));
    assert!(rows[1].can_override);
    assert!(!rows[1].can_detect);
    assert!(
        resolve(&state, detect()).unwrap().is_none(),
        "old logical index must not retarget"
    );
    assert_eq!(
        resolve(
            &state,
            Action::Polarity {
                device: device(),
                axis: 1,
                code: 24,
                value: "auto".into()
            }
        )
        .unwrap(),
        Some(EngineCommand::SetAxisPolarity {
            device: device(),
            axis: 1,
            polarity: None
        })
    );
}

#[test]
fn global_axis_actions_bypass_profile_layout_running_and_readiness_locks() {
    for status in [
        EngineStatus::Stopped,
        EngineStatus::Starting,
        EngineStatus::Running,
    ] {
        for ready in [false, true] {
            let mut state = state();
            state.engine_status = status;
            state.session.ready = ready;
            state.session.captured = vec![device()];
            let rows = prepare(&state, None).controllers.axes;
            assert!(rows[0].can_override);
            assert!(rows[0].can_detect);
            assert_eq!(
                resolve(&state, detect()).unwrap(),
                Some(EngineCommand::DetectAxis {
                    device: device(),
                    axis: 0
                })
            );
            for (value, expected) in [
                ("auto", None),
                ("centered", Some(AxisPolarity::Bipolar)),
                ("minimum", Some(AxisPolarity::Unipolar)),
            ] {
                assert_eq!(
                    resolve(&state, polarity(value)).unwrap(),
                    Some(EngineCommand::SetAxisPolarity {
                        device: device(),
                        axis: 0,
                        polarity: expected
                    })
                );
            }
        }
    }
}

#[test]
fn redetection_requires_monitoring_and_resolving_but_override_only_requires_online() {
    for monitored in [false, true] {
        for available in [false, true] {
            for offline in [false, true] {
                let mut state = state();
                state.session.offline = offline;
                if !monitored {
                    state.session.monitored.clear();
                }
                if !available {
                    state.session.bindings[0]
                        .unavailable
                        .push(InputId::Axis { index: 0 });
                }
                let row = prepare(&state, None).controllers.axes.remove(0);
                assert_eq!(row.can_override, !offline);
                assert_eq!(row.can_detect, !offline && monitored && available);
                assert_eq!(
                    resolve(&state, polarity("auto")).unwrap().is_some(),
                    !offline
                );
                assert_eq!(
                    resolve(&state, detect()).unwrap().is_some(),
                    !offline && monitored && available
                );
            }
        }
    }
}

#[test]
fn stale_callbacks_recheck_offline_monitoring_and_native_identity() {
    let state = RwLock::new(state());
    let (tx, rx) = mpsc::channel();
    dispatch(detect(), &state, &tx).unwrap();
    assert_eq!(
        rx.try_recv().unwrap(),
        EngineCommand::DetectAxis {
            device: device(),
            axis: 0
        }
    );
    state.write().session.bindings[0].axes.swap(0, 1);
    dispatch(polarity("minimum"), &state, &tx).unwrap();
    dispatch(detect(), &state, &tx).unwrap();
    state.write().session.bindings[0].axes.swap(0, 1);
    state.write().session.monitored.clear();
    dispatch(detect(), &state, &tx).unwrap();
    state.write().session.offline = true;
    dispatch(polarity("auto"), &state, &tx).unwrap();
    state.write().session.offline = false;
    state.write().session.bindings.clear();
    dispatch(polarity("auto"), &state, &tx).unwrap();
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
}

#[test]
fn invalid_values_and_malformed_native_tables_use_validation_without_mutation() {
    let mut state = state();
    assert!(
        resolve(&state, polarity("unknown"))
            .unwrap_err()
            .to_string()
            .contains("axis interpretation")
    );
    for table in [binding(&[24, 24]), binding(&vec![24; 256])] {
        state.session.bindings = vec![table];
        assert!(
            resolve(&state, polarity("auto"))
                .unwrap_err()
                .to_string()
                .contains("invalid")
        );
        assert_eq!(prepare(&state, None).controllers.axes, []);
    }
    state.session.bindings = vec![binding(&[24])];
    state.session.bindings[0].axes[0].minimum = 100;
    assert!(
        resolve(&state, detect())
            .unwrap_err()
            .to_string()
            .contains("invalid")
    );
    assert_eq!(prepare(&state, None).controllers.axes, []);
}

#[test]
fn delivery_preserves_settings_and_reports_channel_failure() {
    let state = RwLock::new(state());
    let before = state.read().device_registry.clone();
    let (tx, rx) = mpsc::channel();
    dispatch(polarity("minimum"), &state, &tx).unwrap();
    assert_eq!(
        rx.try_recv().unwrap(),
        EngineCommand::SetAxisPolarity {
            device: device(),
            axis: 0,
            polarity: Some(AxisPolarity::Unipolar)
        }
    );
    assert_eq!(state.read().device_registry, before);
    drop(rx);
    assert!(dispatch(detect(), &state, &tx).is_err());
    assert_eq!(state.read().device_registry, before);
}

#[test]
fn saved_other_devices_are_appended_without_cross_device_native_code_matches() {
    let mut state = state();
    let mut other = binding(&[24]);
    other.device = DeviceId("saved-other-stick".into());
    saved(&mut state, other);
    let rows = prepare(&state, None).controllers.axes;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2].device, DeviceId("saved-other-stick".into()));
    assert_eq!(rows[2].override_polarity, None);
    assert_eq!(rows[2].detected, None);
    assert!(rows[2].can_override);
    assert!(!rows[2].can_detect);
    assert_eq!(
        resolve(
            &state,
            Action::Polarity {
                device: DeviceId("unknown-stick".into()),
                axis: 0,
                code: 24,
                value: "auto".into(),
            }
        )
        .unwrap(),
        None
    );
    assert_eq!(
        resolve(
            &state,
            Action::Polarity {
                device: device(),
                axis: 2,
                code: 24,
                value: "auto".into()
            }
        )
        .unwrap(),
        None
    );
}
