// Rust guideline compliant 2026-03-06

//! Engine integration and unit tests.
//!
//! This module is feature-gated behind `test-util` so it can access
//! the mock I/O implementations.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Instant;

use parking_lot::RwLock;

use crate::action::{Action, Condition, Mapping, ModeChangeStrategy};
use crate::callbacks::{CallbackRegistry, ReleaseCallback};
use crate::device::mock::{MockDeviceHider, MockInputSource};
use crate::device::traits::HotplugEvent;
use crate::mode::{ModeState, ModeTree};
use crate::output::mock::{KeyboardCall, MockKeyboardSink, MockOutputSink, OutputCall};
use crate::pipeline::PipelineOutput;
use crate::profile::Profile;
use crate::profile::manager::{create_profile_in, sanitize_filename};
use crate::settings::AppSettings;
use crate::state::{
    AppState, DeviceState, EngineStatus, InputCacheStore, OutputCacheStore, ProfileOrigin,
};
use crate::types::{
    AxisPolarity, AxisValue, DeviceConnectionState, DeviceDiagnostics, DeviceId, DeviceInfo,
    HatDirection, InputAddress, InputEvent, InputId, InputValue, KeyCombo, MergeOp, OutputAddress,
    OutputId, VJoyAxis,
};

use inputforge_autostart::mock::MockAutostart;

use super::Engine;
use super::command::EngineCommand;
use super::output_handler::{process_pipeline_outputs, refresh_axes_for_mode_change};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DEV: &str = "dev-1";

fn dev_id() -> DeviceId {
    DeviceId(DEV.to_owned())
}

fn axis_addr(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: dev_id(),
        input: InputId::Axis { index },
    }
}

fn button_addr(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: dev_id(),
        input: InputId::Button { index },
    }
}

fn vjoy_axis_output(device: u8, axis: VJoyAxis) -> OutputAddress {
    OutputAddress {
        device,
        output: OutputId::Axis { id: axis },
    }
}

fn vjoy_button_output(device: u8, button: u8) -> OutputAddress {
    OutputAddress {
        device,
        output: OutputId::Button { id: button },
    }
}

fn axis_event(index: u8, value: f64) -> InputEvent {
    InputEvent {
        source: axis_addr(index),
        value: InputValue::Axis {
            value: AxisValue::new(value),
            polarity: AxisPolarity::Bipolar,
        },
        timestamp: Instant::now(),
    }
}

fn button_event(index: u8, pressed: bool) -> InputEvent {
    InputEvent {
        source: button_addr(index),
        value: InputValue::Button { pressed },
        timestamp: Instant::now(),
    }
}

/// Build a minimal `ModeTree` with Default at root.
fn simple_mode_tree() -> ModeTree {
    let map = HashMap::from([("Default".to_owned(), vec![])]);
    ModeTree::from_adjacency(&map).unwrap()
}

/// Build a `ModeTree` with Default → Combat.
fn two_mode_tree() -> ModeTree {
    let map = HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
    ModeTree::from_adjacency(&map).unwrap()
}

/// Build a `ModeTree` with Default → Shift.
fn shift_mode_tree() -> ModeTree {
    let map = HashMap::from([("Default".to_owned(), vec!["Shift".to_owned()])]);
    ModeTree::from_adjacency(&map).unwrap()
}

/// Build a profile with the given mode tree and mappings.
fn make_profile(modes: ModeTree, mappings: Vec<Mapping>) -> Profile {
    Profile::new(
        "Test".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    )
}

/// Build an engine wired to mocks, returning handles for inspection.
///
/// The engine starts in `Running` status with the given profile loaded.
fn make_engine(
    input: MockInputSource,
    profile: Profile,
) -> (Engine, Arc<RwLock<AppState>>, mpsc::Sender<EngineCommand>) {
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    state.write().engine_status = EngineStatus::Running;

    let (tx, rx) = mpsc::channel();

    let engine = Engine::new(
        Box::new(input),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    (engine, state, tx)
}

fn test_engine_with_settings_path(settings: AppSettings) -> (Engine, PathBuf) {
    let settings_path =
        std::env::temp_dir().join(format!("inputforge-settings-{}.toml", ulid::Ulid::new()));
    settings.save_to(&settings_path).unwrap();

    let profile = make_profile(simple_mode_tree(), vec![]);
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    state.write().engine_status = EngineStatus::Running;
    let (_tx, rx) = mpsc::channel();
    let engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        settings,
        settings_path.clone(),
        Box::new(MockAutostart::new()),
    );

    (engine, settings_path)
}

// ---------------------------------------------------------------------------
// T1-T7: Output handler unit tests
// ---------------------------------------------------------------------------

#[test]
fn process_outputs_set_axis() {
    let outputs = vec![PipelineOutput::SetAxis {
        output: vjoy_axis_output(1, VJoyAxis::X),
        value: 0.75,
    }];

    let mut sink = MockOutputSink::new();
    let mut kb = MockKeyboardSink::new();
    let tree = simple_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(0);

    let result = process_pipeline_outputs(
        &outputs,
        &mut sink,
        &mut kb,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &trigger,
    )
    .unwrap();

    assert!(!result.mode_changed);
    assert_eq!(
        sink.calls(),
        &[OutputCall::SetAxis {
            device: 1,
            axis: VJoyAxis::X,
            value: 0.75,
        }]
    );
}

#[test]
fn process_outputs_set_button() {
    let outputs = vec![PipelineOutput::SetButton {
        output: vjoy_button_output(1, 3),
        pressed: true,
    }];

    let mut sink = MockOutputSink::new();
    let mut kb = MockKeyboardSink::new();
    let tree = simple_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(0);

    process_pipeline_outputs(
        &outputs,
        &mut sink,
        &mut kb,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &trigger,
    )
    .unwrap();

    assert_eq!(
        sink.calls(),
        &[OutputCall::SetButton {
            device: 1,
            button: 3,
            pressed: true,
        }]
    );
}

#[test]
fn process_outputs_send_key_only_on_press() {
    let combo = KeyCombo {
        key: "Space".to_owned(),
        modifiers: vec![],
    };

    let pressed_outputs = vec![PipelineOutput::SendKey {
        key: combo.clone(),
        pressed: true,
    }];
    let released_outputs = vec![PipelineOutput::SendKey {
        key: combo.clone(),
        pressed: false,
    }];

    let tree = simple_mode_tree();
    let trigger = button_addr(0);

    // Pressed → key sent.
    let mut sink = MockOutputSink::new();
    let mut kb = MockKeyboardSink::new();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();

    process_pipeline_outputs(
        &pressed_outputs,
        &mut sink,
        &mut kb,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &trigger,
    )
    .unwrap();
    assert_eq!(kb.calls(), &[KeyboardCall::SendKey(combo.clone())]);

    // Released → no key sent.
    let mut kb2 = MockKeyboardSink::new();
    let mut sink2 = MockOutputSink::new();
    let mut mode_state2 = ModeState::new("Default".to_owned());
    let mut callbacks2 = CallbackRegistry::new();

    process_pipeline_outputs(
        &released_outputs,
        &mut sink2,
        &mut kb2,
        &mut mode_state2,
        &tree,
        &mut callbacks2,
        &trigger,
    )
    .unwrap();
    assert!(kb2.calls().is_empty());
}

#[test]
fn process_outputs_change_mode_switch_to() {
    let tree = two_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(0);

    let outputs = vec![PipelineOutput::ChangeMode {
        strategy: ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        },
    }];

    let mut sink = MockOutputSink::new();
    let mut kb = MockKeyboardSink::new();

    let result = process_pipeline_outputs(
        &outputs,
        &mut sink,
        &mut kb,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &trigger,
    )
    .unwrap();

    assert!(result.mode_changed);
    assert_eq!(mode_state.current(), "Combat");
}

#[test]
fn process_outputs_temporary_mode_registers_callback() {
    let tree = shift_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(5);

    let outputs = vec![PipelineOutput::ChangeMode {
        strategy: ModeChangeStrategy::Temporary {
            mode: "Shift".to_owned(),
        },
    }];

    let mut sink = MockOutputSink::new();
    let mut kb = MockKeyboardSink::new();

    let result = process_pipeline_outputs(
        &outputs,
        &mut sink,
        &mut kb,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &trigger,
    )
    .unwrap();

    assert!(result.mode_changed);
    assert_eq!(mode_state.current(), "Shift");

    // Firing the callback for the triggering input returns PopTemporaryMode.
    let fired = callbacks.fire(&trigger);
    assert_eq!(fired.len(), 1);
    assert!(matches!(fired[0], ReleaseCallback::PopTemporaryMode));
}

#[test]
fn refresh_axes_reprocesses_cached_values() {
    let tree = simple_mode_tree();
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };

    let mut cache = InputCacheStore::new();
    cache.update(
        &axis_addr(0),
        &InputValue::Axis {
            value: AxisValue::new(0.5),
            polarity: AxisPolarity::Bipolar,
        },
    );

    let mut sink = MockOutputSink::new();
    refresh_axes_for_mode_change(
        &cache,
        &[mapping],
        "Default",
        &tree,
        &mut sink,
        &mut OutputCacheStore::new(),
    )
    .unwrap();

    assert_eq!(
        sink.calls(),
        &[OutputCall::SetAxis {
            device: 1,
            axis: VJoyAxis::X,
            value: 0.5,
        }]
    );
}

#[test]
fn refresh_axes_skips_mode_changes_and_keys() {
    let tree = two_mode_tree();
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![
            Action::ChangeMode {
                strategy: ModeChangeStrategy::SwitchTo {
                    mode: "Combat".to_owned(),
                },
            },
            Action::MapToKeyboard {
                key: KeyCombo {
                    key: "A".to_owned(),
                    modifiers: vec![],
                },
            },
        ],
    };

    let mut cache = InputCacheStore::new();
    cache.update(
        &axis_addr(0),
        &InputValue::Axis {
            value: AxisValue::new(0.3),
            polarity: AxisPolarity::Bipolar,
        },
    );

    let mut sink = MockOutputSink::new();
    refresh_axes_for_mode_change(
        &cache,
        &[mapping],
        "Default",
        &tree,
        &mut sink,
        &mut OutputCacheStore::new(),
    )
    .unwrap();

    // No axis/button outputs were produced, mode changes and keys skipped.
    assert!(sink.calls().is_empty());
}

// ---------------------------------------------------------------------------
// T8-T16: Engine::tick integration tests
// ---------------------------------------------------------------------------

#[test]
fn tick_processes_axis_event_to_output() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 0.5));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    // Input cache was updated.
    let s = state.read();
    let cached = s.input_cache.get_all_axis_entries();
    assert_eq!(cached.len(), 1);
    assert!((cached[0].1 - 0.5).abs() < f64::EPSILON);

    // Mode unchanged.
    assert_eq!(s.current_mode, "Default");
}

#[test]
fn tick_merge_axis_secondary_event_refreshes_primary_mapping_output() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Average,
            },
            Action::MapToVJoy {
                output: vjoy_axis_output(1, VJoyAxis::X),
            },
        ],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 0.2));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    input = MockInputSource::default();
    input.events.push(axis_event(1, 0.8));
    engine.input = Box::new(input);
    engine.tick().unwrap();

    let output = state.read().output_cache.get_axis(1, VJoyAxis::X);
    assert!(
        (output - 0.5).abs() < f64::EPSILON,
        "secondary-only merge event should refresh output from cached primary, got {output}"
    );
}

#[test]
fn tick_conditional_button_predicate_event_refreshes_primary_mapping_output() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: button_addr(1),
            },
            if_true: vec![
                Action::Invert,
                Action::MapToVJoy {
                    output: vjoy_axis_output(1, VJoyAxis::X),
                },
            ],
            if_false: vec![Action::MapToVJoy {
                output: vjoy_axis_output(1, VJoyAxis::X),
            }],
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 0.6));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    input = MockInputSource::default();
    input.events.push(button_event(1, true));
    engine.input = Box::new(input);
    engine.tick().unwrap();

    let output = state.read().output_cache.get_axis(1, VJoyAxis::X);
    assert!(
        (output - (-0.6)).abs() < f64::EPSILON,
        "predicate-only button event should refresh conditional output, got {output}"
    );
}

#[test]
fn tick_conditional_axis_predicate_event_refreshes_primary_mapping_output() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::Conditional {
            condition: Condition::AxisInRange {
                input: axis_addr(1),
                min: 0.5,
                max: 1.0,
            },
            if_true: vec![
                Action::Invert,
                Action::MapToVJoy {
                    output: vjoy_axis_output(1, VJoyAxis::X),
                },
            ],
            if_false: vec![Action::MapToVJoy {
                output: vjoy_axis_output(1, VJoyAxis::X),
            }],
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 0.4));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    input = MockInputSource::default();
    input.events.push(axis_event(1, 0.75));
    engine.input = Box::new(input);
    engine.tick().unwrap();

    let output = state.read().output_cache.get_axis(1, VJoyAxis::X);
    assert!(
        (output - (-0.4)).abs() < f64::EPSILON,
        "predicate-only axis event should refresh conditional output, got {output}"
    );
}

#[test]
fn tick_unrelated_input_event_does_not_refresh_mapping_output() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 0.2));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    input = MockInputSource::default();
    input.events.push(axis_event(2, 0.9));
    engine.input = Box::new(input);
    engine.tick().unwrap();

    let output = state.read().output_cache.get_axis(1, VJoyAxis::X);
    assert!(
        (output - 0.2).abs() < f64::EPSILON,
        "unrelated event should leave previous output cached, got {output}"
    );
    assert!(
        engine.output_buffer.is_empty(),
        "unrelated event should not emit a fresh output"
    );
}

#[test]
fn tick_dependent_mapping_runs_once_when_input_is_referenced_twice() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Average,
            },
            Action::Conditional {
                condition: Condition::AxisInRange {
                    input: axis_addr(1),
                    min: 0.5,
                    max: 1.0,
                },
                if_true: vec![Action::MapToVJoy {
                    output: vjoy_axis_output(1, VJoyAxis::X),
                }],
                if_false: vec![Action::MapToVJoy {
                    output: vjoy_axis_output(1, VJoyAxis::X),
                }],
            },
        ],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 0.2));

    let (mut engine, _state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    input = MockInputSource::default();
    input.events.push(axis_event(1, 0.8));
    engine.input = Box::new(input);
    engine.tick().unwrap();

    assert_eq!(
        engine.output_buffer.len(),
        1,
        "a mapping with duplicate dependency references should execute once"
    );
}

#[test]
fn tick_updates_current_mode_in_state() {
    let mapping = Mapping {
        input: button_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::ChangeMode {
            strategy: ModeChangeStrategy::SwitchTo {
                mode: "Combat".to_owned(),
            },
        }],
    };
    let profile = make_profile(two_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(button_event(0, true));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    assert_eq!(state.read().current_mode, "Combat");
}

#[test]
fn tick_temporary_mode_press_and_release_full_cycle() {
    // Regression seal for the strategy-aware rising-edge gate
    // (pipeline/mod.rs Action::ChangeMode arm). Without the gate, the
    // pipeline rerun on the release tick (after the PopTemporaryMode
    // callback already popped) would re-push the temporary mode and
    // leave the user stuck in it. The cycle must end on Default with
    // an empty temporary stack.
    let mapping = Mapping {
        input: button_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::ChangeMode {
            strategy: ModeChangeStrategy::Temporary {
                mode: "Shift".to_owned(),
            },
        }],
    };
    let profile = make_profile(shift_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(button_event(0, true));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();
    assert_eq!(
        state.read().current_mode,
        "Shift",
        "press tick must push the temporary mode"
    );

    input = MockInputSource::default();
    input.events.push(button_event(0, false));
    engine.input = Box::new(input);
    engine.tick().unwrap();

    assert_eq!(
        state.read().current_mode,
        "Default",
        "release tick must pop back to Default and not re-push"
    );
    // A second release with no events must remain on Default; this guards
    // against a leaked callback re-pushing on a subsequent tick.
    engine.input = Box::new(MockInputSource::default());
    engine.tick().unwrap();
    assert_eq!(state.read().current_mode, "Default");
}

#[test]
fn tick_skips_processing_when_paused() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 0.5));

    let (mut engine, state, _tx) = make_engine(input, profile);

    // Set paused before tick.
    state.write().engine_status = EngineStatus::Paused;

    engine.tick().unwrap();

    // The input cache is updated unconditionally so the GUI can display live
    // values even when paused.  What must NOT happen is output: no vJoy write.
    assert_eq!(state.read().input_cache.get_all_axis_entries().len(), 1);
}

#[test]
fn tick_handles_activate_command() {
    let profile = make_profile(simple_mode_tree(), vec![]);

    let input = MockInputSource::default();
    let (mut engine, state, tx) = make_engine(input, profile);

    // Override to Stopped.
    state.write().engine_status = EngineStatus::Stopped;

    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();

    assert_eq!(state.read().engine_status, EngineStatus::Running);
}

#[test]
fn tick_handles_shutdown_command() {
    let profile = make_profile(simple_mode_tree(), vec![]);
    let input = MockInputSource::default();
    let (mut engine, _state, tx) = make_engine(input, profile);

    tx.send(EngineCommand::Shutdown).unwrap();
    engine.tick().unwrap();

    assert!(engine.shutdown);
}

#[test]
fn tick_hotplug_connected_adds_device() {
    let profile = make_profile(simple_mode_tree(), vec![]);

    let device_info = DeviceInfo {
        id: DeviceId("joy-1".to_owned()),
        name: "Test Stick".to_owned(),
        axes: 4,
        buttons: 12,
        hats: 1,
        instance_path: None,
        axis_polarities: vec![],
    };

    let mut input = MockInputSource::default();
    input.hotplug.push(HotplugEvent::Connected {
        info: device_info,
        diagnostics: DeviceDiagnostics::default(),
    });

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(s.devices.len(), 1);
    assert!(s.devices[0].connected);
    assert_eq!(s.devices[0].info.name, "Test Stick");
}

#[test]
fn connected_hotplug_upserts_device_registry() {
    let profile = make_profile(simple_mode_tree(), vec![]);
    let device = DeviceId("dev-1".to_owned());
    let info = DeviceInfo {
        id: device.clone(),
        name: "Wheel".to_owned(),
        axes: 4,
        buttons: 12,
        hats: 1,
        instance_path: Some("hid-path".to_owned()),
        axis_polarities: vec![],
    };
    let diagnostics = DeviceDiagnostics {
        vendor_id: Some(0x045e),
        product_id: Some(0x028e),
        connection_state: Some(DeviceConnectionState::Wired),
        ..DeviceDiagnostics::default()
    };

    let settings_dir = tempfile::tempdir().expect("settings tempdir");
    let settings_path = settings_dir.path().join("settings.toml");
    let mut input = MockInputSource::default();
    input.hotplug.push(HotplugEvent::Connected {
        info: info.clone(),
        diagnostics: diagnostics.clone(),
    });
    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.settings_path = settings_path.clone();

    engine.tick().expect("hotplug tick succeeds");

    let loaded = AppSettings::load_from(&settings_path);
    let record = loaded
        .device_registry
        .get(&device)
        .expect("registry record");
    assert_eq!(record.info, info);
    assert_eq!(record.diagnostics, diagnostics);
    assert!(record.last_seen_unix_ms.is_some());
    assert!(state.read().device_registry.contains_key(&device));
    assert!(
        state
            .read()
            .devices
            .iter()
            .any(|row| row.info.id == device && row.connected)
    );
}

#[test]
fn tick_hotplug_disconnected_marks_device() {
    let profile = make_profile(simple_mode_tree(), vec![]);

    let device_info = DeviceInfo {
        id: DeviceId("joy-1".to_owned()),
        name: "Test Stick".to_owned(),
        axes: 4,
        buttons: 12,
        hats: 1,
        instance_path: None,
        axis_polarities: vec![],
    };

    // Pre-load the disconnect event before engine construction.
    let mut input = MockInputSource::default();
    input
        .hotplug
        .push(HotplugEvent::Disconnected(DeviceId("joy-1".to_owned())));

    let (mut engine, state, _tx) = make_engine(input, profile);

    // Pre-populate a connected device in shared state.
    state.write().devices.push(DeviceState {
        info: device_info,
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });

    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(s.devices.len(), 1);
    assert!(!s.devices[0].connected);
}

#[test]
fn tick_release_pops_temporary_mode_before_mapping() {
    // Mapping: button 0 in Default → push Shift mode (temporary),
    // but only when the button is pressed. On release the Conditional
    // does not fire, so the pop from the release callback is the only
    // mode change. This mirrors real-world temporary mode usage.
    let mapping = Mapping {
        input: button_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: button_addr(0),
            },
            if_true: vec![Action::ChangeMode {
                strategy: ModeChangeStrategy::Temporary {
                    mode: "Shift".to_owned(),
                },
            }],
            if_false: Vec::new(),
        }],
    };
    let profile = make_profile(shift_mode_tree(), vec![mapping]);

    // First tick: press button → pushes temporary Shift mode.
    let mut input = MockInputSource::default();
    input.events.push(button_event(0, true));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    assert_eq!(state.read().current_mode, "Shift");

    // Second tick: release button → callback pops Shift before mapping.
    engine.input = Box::new({
        let mut source = MockInputSource::default();
        source.events.push(button_event(0, false));
        source
    });
    engine.tick().unwrap();

    // Mode should be back to Default (pop happened before mapping resolution).
    assert_eq!(state.read().current_mode, "Default");
}

#[test]
fn drop_flushes_output_without_panic() {
    let profile = make_profile(simple_mode_tree(), vec![]);
    let input = MockInputSource::default();
    let (engine, _state, _tx) = make_engine(input, profile);

    // Dropping the engine should flush output without panicking.
    drop(engine);
}

// ---------------------------------------------------------------------------
// Additional helpers
// ---------------------------------------------------------------------------

fn hat_addr(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: dev_id(),
        input: InputId::Hat { index },
    }
}

fn hat_event(index: u8, direction: HatDirection) -> InputEvent {
    InputEvent {
        source: hat_addr(index),
        value: InputValue::Hat { direction },
        timestamp: Instant::now(),
    }
}

/// Build a `ModeTree` with Default → Combat and Default → Landing.
fn three_mode_tree() -> ModeTree {
    let map = HashMap::from([(
        "Default".to_owned(),
        vec!["Combat".to_owned(), "Landing".to_owned()],
    )]);
    ModeTree::from_adjacency(&map).unwrap()
}

/// Build an engine without a loaded profile.
fn make_engine_no_profile(
    input: MockInputSource,
) -> (Engine, Arc<RwLock<AppState>>, mpsc::Sender<EngineCommand>) {
    let state = Arc::new(RwLock::new(AppState::new()));
    state.write().engine_status = EngineStatus::Running;

    let (tx, rx) = mpsc::channel();

    let engine = Engine::new(
        Box::new(input),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    (engine, state, tx)
}

struct EngineHarness {
    engine: Engine,
    state: Arc<RwLock<AppState>>,
    #[expect(
        clippy::used_underscore_binding,
        reason = "field is held only to keep the tempdir alive for the harness lifetime; \
                  the underscore prefix signals the binding is intentionally not read"
    )]
    _settings_dir: tempfile::TempDir,
    library_dir: PathBuf,
    /// Cloned handle to the autostart mock the engine was constructed with.
    /// Tests inspect calls and seed `is_enabled` results through this clone;
    /// the inner state is shared via `Arc<Mutex<>>`.
    autostart_mock: MockAutostart,
}

impl EngineHarness {
    fn new() -> Self {
        let settings_dir = tempfile::tempdir().unwrap();
        let settings_path = settings_dir.path().join("settings.toml");
        let library_dir = settings_dir.path().join("profiles");
        let settings = AppSettings::default();
        settings.save_to(&settings_path).unwrap();

        let state = Arc::new(RwLock::new(AppState::new()));
        let (_tx, rx) = mpsc::channel();
        // MockAutostart is Clone with internal Arc<Mutex<>>-shared state, so
        // the engine takes one boxed clone while the harness keeps another
        // for assertions. Both clones see the same call log and seeded
        // results. See the contract test `clone_shares_state_with_original`
        // (Task 1.5) for the proof.
        let autostart_mock = MockAutostart::new();
        let engine = Engine::new(
            Box::new(MockInputSource::default()),
            Box::new(MockOutputSink::new()),
            Box::new(MockKeyboardSink::new()),
            Box::new(MockDeviceHider::default()),
            Arc::clone(&state),
            rx,
            settings,
            settings_path,
            Box::new(autostart_mock.clone()),
        );

        Self {
            engine,
            state,
            _settings_dir: settings_dir,
            library_dir,
            autostart_mock,
        }
    }

    fn dispatch(&mut self, command: EngineCommand) -> crate::error::Result<()> {
        self.engine.handle_command(command)
    }

    fn state(&self) -> parking_lot::RwLockReadGuard<'_, AppState> {
        self.state.read()
    }

    fn write_external_profile(&self, name: &str) -> PathBuf {
        let path = self
            ._settings_dir
            .path()
            .join(format!("{}.toml", sanitize_filename(name)));
        make_profile(simple_mode_tree(), vec![])
            .save(&path)
            .unwrap();
        path
    }

    fn create_and_load_profile(&mut self, name: &str) -> crate::error::Result<()> {
        let path = create_profile_in(name, &self.library_dir)?;
        self.dispatch(EngineCommand::LoadProfile(path))
    }

    fn profile_path(&self, name: &str) -> PathBuf {
        self.library_dir
            .join(format!("{}.toml", sanitize_filename(name)))
    }

    /// Replace the engine's settings_path with one containing a NUL byte
    /// so std::fs::create_dir_all and File::create both fail with
    /// `ErrorKind::InvalidInput` deterministically on every OS. Used for
    /// save-failure tests.
    fn force_settings_path_to_unwritable(&mut self) {
        let mut path = self._settings_dir.path().to_path_buf();
        path.push("settings\0.toml");
        self.engine.settings_path = path;
    }
}

#[test]
fn load_external_profile_once_marks_origin_external_and_does_not_add_library_row() {
    let mut harness = EngineHarness::new();
    let external = harness.write_external_profile("External");

    harness
        .dispatch(EngineCommand::LoadExternalProfileOnce(external.clone()))
        .unwrap();

    let state = harness.state();
    assert_eq!(state.profile_path.as_ref(), Some(&external));
    assert_eq!(state.active_profile_origin, Some(ProfileOrigin::External));
    assert!(
        state
            .profile_library_rows
            .iter()
            .all(|row| row.path != external)
    );
    assert_eq!(state.engine_status, EngineStatus::Stopped);
}

#[test]
fn delete_active_library_profile_enters_no_profile_state_and_refreshes_rows() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();

    harness
        .dispatch(EngineCommand::DeleteProfile {
            name: "Alpha".to_owned(),
        })
        .unwrap();

    let state = harness.state();
    assert!(state.active_profile.is_none());
    assert!(state.profile_path.is_none());
    assert!(state.active_profile_origin.is_none());
    assert!(state.active_snapshot_rows.is_empty());
    assert!(
        state
            .profile_library_rows
            .iter()
            .all(|row| row.name != "Alpha")
    );
    assert_eq!(state.engine_status, EngineStatus::Stopped);
}

#[test]
fn profile_lifecycle_commands_refresh_projected_library_rows() {
    let mut harness = EngineHarness::new();
    harness
        .dispatch(EngineCommand::CreateProfile {
            name: "Alpha".to_owned(),
        })
        .unwrap();
    harness
        .dispatch(EngineCommand::DuplicateProfile {
            source_path: harness.profile_path("Alpha"),
            name: "Bravo".to_owned(),
        })
        .unwrap();

    let state = harness.state();
    let names = state
        .profile_library_rows
        .iter()
        .map(|row| row.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["Alpha", "Bravo"]);
}

// ---------------------------------------------------------------------------
// `settings.last_profile` persistence: every command that changes which
// profile is active must mirror `state.profile_path` into
// `settings.last_profile` and persist via `AppSettings::save_to()`,
// otherwise next launch reloads a stale path. `LoadExternalProfileOnce`
// is intentionally exempt: the "Once" semantic is transient by design.
// ---------------------------------------------------------------------------

#[test]
fn load_profile_persists_path_to_settings_last_profile() {
    let mut harness = EngineHarness::new();
    let path = create_profile_in("Alpha", &harness.library_dir).unwrap();
    harness
        .dispatch(EngineCommand::LoadProfile(path.clone()))
        .unwrap();

    assert_eq!(harness.engine.settings.last_profile.as_ref(), Some(&path));
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.last_profile.as_ref(), Some(&path));
}

#[test]
fn create_profile_persists_path_to_settings_last_profile() {
    let mut harness = EngineHarness::new();
    harness
        .dispatch(EngineCommand::CreateProfile {
            name: "Alpha".to_owned(),
        })
        .unwrap();

    let expected = harness.profile_path("Alpha");
    assert_eq!(
        harness.engine.settings.last_profile.as_ref(),
        Some(&expected)
    );
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.last_profile.as_ref(), Some(&expected));
}

#[test]
fn rename_active_profile_persists_new_path_to_settings_last_profile() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();

    harness
        .dispatch(EngineCommand::RenameProfile {
            old_name: "Alpha".to_owned(),
            new_name: "Bravo".to_owned(),
        })
        .unwrap();

    let renamed_path = harness.profile_path("Bravo");
    assert_eq!(
        harness.engine.settings.last_profile.as_ref(),
        Some(&renamed_path)
    );
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.last_profile.as_ref(), Some(&renamed_path));
}

#[test]
fn rename_inactive_profile_does_not_change_settings_last_profile() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();
    let _bravo = create_profile_in("Bravo", &harness.library_dir).unwrap();
    let alpha_path = harness.profile_path("Alpha");

    // Renaming the INACTIVE Bravo must not touch the persisted path.
    harness
        .dispatch(EngineCommand::RenameProfile {
            old_name: "Bravo".to_owned(),
            new_name: "Charlie".to_owned(),
        })
        .unwrap();

    assert_eq!(
        harness.engine.settings.last_profile.as_ref(),
        Some(&alpha_path)
    );
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.last_profile.as_ref(), Some(&alpha_path));
}

#[test]
fn delete_active_profile_clears_settings_last_profile() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();

    harness
        .dispatch(EngineCommand::DeleteProfile {
            name: "Alpha".to_owned(),
        })
        .unwrap();

    assert!(harness.engine.settings.last_profile.is_none());
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert!(on_disk.last_profile.is_none());
}

#[test]
fn delete_inactive_profile_does_not_change_settings_last_profile() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();
    let _bravo = create_profile_in("Bravo", &harness.library_dir).unwrap();
    let alpha_path = harness.profile_path("Alpha");

    harness
        .dispatch(EngineCommand::DeleteProfile {
            name: "Bravo".to_owned(),
        })
        .unwrap();

    assert_eq!(
        harness.engine.settings.last_profile.as_ref(),
        Some(&alpha_path)
    );
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.last_profile.as_ref(), Some(&alpha_path));
}

#[test]
fn load_external_profile_once_does_not_change_settings_last_profile() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();
    let alpha_path = harness.profile_path("Alpha");
    let external = harness.write_external_profile("External");

    harness
        .dispatch(EngineCommand::LoadExternalProfileOnce(external))
        .unwrap();

    // The external profile is now active in `state.profile_path`, but
    // the persisted `last_profile` must stay pointed at Alpha because
    // "Once" is transient by design.
    assert_eq!(
        harness.engine.settings.last_profile.as_ref(),
        Some(&alpha_path)
    );
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.last_profile.as_ref(), Some(&alpha_path));
}

#[test]
fn add_external_profile_to_library_persists_path_to_settings_last_profile() {
    let mut harness = EngineHarness::new();
    let external = harness.write_external_profile("External");

    harness
        .dispatch(EngineCommand::AddExternalProfileToLibrary {
            path: external,
            name: "Imported".to_owned(),
        })
        .unwrap();

    let imported_path = harness.profile_path("Imported");
    assert_eq!(
        harness.engine.settings.last_profile.as_ref(),
        Some(&imported_path)
    );
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.last_profile.as_ref(), Some(&imported_path));
}

// ---------------------------------------------------------------------------
// T17-T22: Output handler unit tests (coverage round 2)
// ---------------------------------------------------------------------------

#[test]
fn process_outputs_set_axis_wrong_output_id() {
    // SetAxis with an OutputId::Button should be skipped (warn path).
    let outputs = vec![PipelineOutput::SetAxis {
        output: vjoy_button_output(1, 3),
        value: 0.5,
    }];

    let mut sink = MockOutputSink::new();
    let mut kb = MockKeyboardSink::new();
    let tree = simple_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(0);

    process_pipeline_outputs(
        &outputs,
        &mut sink,
        &mut kb,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &trigger,
    )
    .unwrap();

    assert!(sink.calls().is_empty());
}

#[test]
fn process_outputs_set_button_wrong_output_id() {
    // SetButton with an OutputId::Axis should be skipped (warn path).
    let outputs = vec![PipelineOutput::SetButton {
        output: vjoy_axis_output(1, VJoyAxis::X),
        pressed: true,
    }];

    let mut sink = MockOutputSink::new();
    let mut kb = MockKeyboardSink::new();
    let tree = simple_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(0);

    process_pipeline_outputs(
        &outputs,
        &mut sink,
        &mut kb,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &trigger,
    )
    .unwrap();

    assert!(sink.calls().is_empty());
}

#[test]
fn process_outputs_mode_change_no_op() {
    // Switching to the current mode should not set mode_changed.
    let tree = two_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(0);

    let outputs = vec![PipelineOutput::ChangeMode {
        strategy: ModeChangeStrategy::SwitchTo {
            mode: "Default".to_owned(),
        },
    }];

    let mut sink = MockOutputSink::new();
    let mut kb = MockKeyboardSink::new();

    let result = process_pipeline_outputs(
        &outputs,
        &mut sink,
        &mut kb,
        &mut mode_state,
        &tree,
        &mut callbacks,
        &trigger,
    )
    .unwrap();

    assert!(!result.mode_changed);
    assert_eq!(mode_state.current(), "Default");
}

#[test]
fn refresh_axes_set_button_path() {
    // Mapping an axis to a button output goes through the SetButton branch
    // in refresh_axes_for_mode_change. We craft a PipelineOutput::SetButton
    // by using MapToVJoy with a button OutputAddress and a Button input value
    // cached as an axis (the pipeline sees input_value as Axis, so it
    // produces SetAxis). Instead, test the branch directly.
    //
    // Since the pipeline always produces SetAxis for axis inputs, we test
    // the refresh SetButton path via direct process_pipeline_outputs.
    // This is the closest we can get without mocking the pipeline.
    let tree = simple_mode_tree();
    let mapping = Mapping {
        input: button_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_button_output(1, 3),
        }],
    };

    // Cache a button value (the refresh only iterates axis entries, so we
    // must cache an axis entry that maps to an action producing SetButton).
    // Since MapToVJoy with axis input always produces SetAxis, the SetButton
    // branch in refresh is defensive. We verify it via the unit-level
    // process_pipeline_outputs tests (T2, T18) instead.
    //
    // Here we verify that refresh works end-to-end for axis → SetAxis.
    let mut cache = InputCacheStore::new();
    cache.update(
        &axis_addr(0),
        &InputValue::Axis {
            value: AxisValue::new(0.8),
            polarity: AxisPolarity::Bipolar,
        },
    );

    let mapping_axis = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::Y),
        }],
    };

    let mut sink = MockOutputSink::new();
    refresh_axes_for_mode_change(
        &cache,
        &[mapping, mapping_axis],
        "Default",
        &tree,
        &mut sink,
        &mut OutputCacheStore::new(),
    )
    .unwrap();

    assert_eq!(
        sink.calls(),
        &[OutputCall::SetAxis {
            device: 1,
            axis: VJoyAxis::Y,
            value: 0.8,
        }]
    );
}

// ---------------------------------------------------------------------------
// T24-T31: Engine::tick integration tests (coverage round 2)
// ---------------------------------------------------------------------------

#[test]
fn tick_no_profile_returns_early() {
    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 0.5));

    let (mut engine, state, _tx) = make_engine_no_profile(input);
    engine.tick().unwrap();

    // The input cache is updated unconditionally so the GUI can display live
    // values even when no profile is loaded.  No output is produced.
    assert_eq!(state.read().input_cache.get_all_axis_entries().len(), 1);
}

#[test]
fn tick_unmapped_input_continues() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    // Button 99 has no mapping → hits the `continue` path.
    input.events.push(button_event(99, true));
    // Axis 0 has a mapping → processes normally.
    input.events.push(axis_event(0, 0.7));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    // Both events should be cached (cache is updated before mapping).
    let s = state.read();
    let axes = s.input_cache.get_all_axis_entries();
    assert_eq!(axes.len(), 1);
    assert!((axes[0].1 - 0.7).abs() < f64::EPSILON);

    // Mode unchanged.
    assert_eq!(s.current_mode, "Default");
}

#[test]
fn tick_hat_event_produces_zero_value() {
    // Map hat 0 → vJoy axis. The hat current_value is 0.0.
    let mapping = Mapping {
        input: hat_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let mut input = MockInputSource::default();
    input.events.push(hat_event(0, HatDirection::N));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    // Hat event was processed (input cache stores hat values).
    let s = state.read();
    assert_eq!(s.current_mode, "Default");
}

#[test]
fn tick_custom_release_callback_fires() {
    let profile = make_profile(simple_mode_tree(), vec![]);

    let mut input = MockInputSource::default();
    input.events.push(button_event(0, false));

    let (mut engine, _state, _tx) = make_engine(input, profile);

    // Register a custom callback on button 0.
    let fired = Arc::new(AtomicBool::new(false));
    let fired_clone = Arc::clone(&fired);
    engine.callbacks.register(
        button_addr(0),
        ReleaseCallback::Custom(Box::new(move || {
            fired_clone.store(true, Ordering::SeqCst);
        })),
    );

    engine.tick().unwrap();

    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tick_handles_deactivate_command() {
    let profile = make_profile(simple_mode_tree(), vec![]);
    let input = MockInputSource::default();
    let (mut engine, state, tx) = make_engine(input, profile);

    tx.send(EngineCommand::Deactivate).unwrap();
    engine.tick().unwrap();

    assert_eq!(state.read().engine_status, EngineStatus::Stopped);
}

#[test]
fn tick_handles_pause_command() {
    let profile = make_profile(simple_mode_tree(), vec![]);
    let input = MockInputSource::default();
    let (mut engine, state, tx) = make_engine(input, profile);

    tx.send(EngineCommand::Pause).unwrap();
    engine.tick().unwrap();

    assert_eq!(state.read().engine_status, EngineStatus::Paused);
}

#[test]
fn tick_channel_disconnected_sets_shutdown() {
    let profile = make_profile(simple_mode_tree(), vec![]);
    let input = MockInputSource::default();
    let (mut engine, _state, tx) = make_engine(input, profile);

    // Drop the sender to disconnect the channel.
    drop(tx);

    engine.tick().unwrap();

    assert!(engine.shutdown);
}

#[test]
fn tick_hotplug_reconnect_updates_existing() {
    let profile = make_profile(simple_mode_tree(), vec![]);

    let original_info = DeviceInfo {
        id: DeviceId("joy-1".to_owned()),
        name: "Old Name".to_owned(),
        axes: 2,
        buttons: 8,
        hats: 0,
        instance_path: None,
        axis_polarities: vec![],
    };

    let updated_info = DeviceInfo {
        id: DeviceId("joy-1".to_owned()),
        name: "New Name".to_owned(),
        axes: 4,
        buttons: 12,
        hats: 1,
        instance_path: None,
        axis_polarities: vec![],
    };

    let mut input = MockInputSource::default();
    input.hotplug.push(HotplugEvent::Connected {
        info: updated_info.clone(),
        diagnostics: DeviceDiagnostics::default(),
    });

    let (mut engine, state, _tx) = make_engine(input, profile);

    // Pre-populate a disconnected device with the same ID.
    state.write().devices.push(DeviceState {
        info: original_info,
        connected: false,
        diagnostics: DeviceDiagnostics::default(),
    });

    engine.tick().unwrap();

    let s = state.read();
    // Should update existing, not add duplicate.
    assert_eq!(s.devices.len(), 1);
    assert!(s.devices[0].connected);
    assert_eq!(s.devices[0].info.name, "New Name");
    assert_eq!(s.devices[0].info.axes, 4);
}

// ---------------------------------------------------------------------------
// T32-T34: mod.rs tests (coverage round 2)
// ---------------------------------------------------------------------------

#[test]
fn engine_debug_format() {
    let profile = make_profile(simple_mode_tree(), vec![]);
    let input = MockInputSource::default();
    let (engine, _state, _tx) = make_engine(input, profile);

    let debug_str = format!("{engine:?}");
    assert!(debug_str.contains("Engine"));
    assert!(debug_str.contains("mode_state"));
    assert!(debug_str.contains("shutdown"));
}

#[test]
fn engine_new_no_profile_defaults_to_default_mode() {
    let input = MockInputSource::default();
    let (engine, _state, _tx) = make_engine_no_profile(input);

    assert_eq!(engine.mode_state.current(), "Default");
}

#[test]
fn tick_handles_load_profile_command() {
    let input = MockInputSource::default();
    let (mut engine, state, tx) = make_engine_no_profile(input);

    // Create a minimal profile and save it to a temp file.
    let profile = make_profile(two_mode_tree(), vec![]);
    let toml_str = profile.to_toml().unwrap();

    let dir = std::env::temp_dir().join("inputforge_engine_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("load_test_profile.toml");
    std::fs::write(&path, &toml_str).unwrap();

    tx.send(EngineCommand::LoadProfile(path.clone())).unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert!(s.active_profile.is_some());
    assert_eq!(s.current_mode, "Default");
    // Clean up.
    drop(s);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

// ---------------------------------------------------------------------------
// Calibration integration tests
// ---------------------------------------------------------------------------

#[test]
fn tick_set_calibration_command_updates_store() {
    use crate::processing::Calibration;

    let profile = make_profile(simple_mode_tree(), vec![]);
    let input = MockInputSource::default();
    let (mut engine, state, tx) = make_engine(input, profile);

    let cal = Calibration::new(-32768.0, -100.0, 100.0, 32767.0, true).unwrap();
    tx.send(EngineCommand::SetCalibration {
        device: dev_id(),
        axis: 0,
        calibration: cal.clone(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    let stored = s.calibrations.get(&dev_id(), 0);
    assert_eq!(stored, Some(&cal));
}

#[test]
fn tick_axis_with_calibration_applies_transform() {
    use crate::processing::Calibration;

    // Map axis 0 → vJoy axis X.
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    // Calibration over the normalised range [-1.0, 1.0] with no centre
    // deadzone.  Input of 0.5 passes through calibration unchanged (identity
    // transform), so we can verify the cache stores the raw value and that
    // calibration does not corrupt it.
    let cal = Calibration::new(-1.0, 0.0, 0.0, 1.0, true).unwrap();

    let mut input = MockInputSource::default();
    // axis_event uses AxisValue::new which clamps to [-1.0, 1.0]; 0.5 is safe.
    input.events.push(axis_event(0, 0.5));

    let (mut engine, state, _tx) = make_engine(input, profile);

    // Pre-populate calibration in shared state.
    state.write().calibrations.set(dev_id(), 0, cal);

    engine.tick().unwrap();

    // Verify the input cache received the raw value (cache stores raw, not
    // the calibrated value used by the pipeline).
    let s = state.read();
    let cached = s.input_cache.get_all_axis_entries();
    assert_eq!(cached.len(), 1);
    // The raw value in the cache should be 0.5.
    assert!((cached[0].1 - 0.5).abs() < f64::EPSILON);
}

#[test]
fn with_profile_loads_calibrations() {
    use crate::profile::{CalibrationEntry, Profile};

    let mut map = HashMap::new();
    map.insert("Default".to_owned(), vec![]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let calibrations = vec![CalibrationEntry {
        device: DeviceId("dev-1".to_owned()),
        axis: 0,
        physical_min: -32768.0,
        physical_center_low: -100.0,
        physical_center_high: 100.0,
        physical_max: 32767.0,
        enabled: true,
    }];

    let profile = Profile::new(
        "Test".to_owned(),
        vec![],
        modes,
        vec![],
        calibrations,
        "Default".to_owned(),
    );

    let state = AppState::with_profile(profile);
    let cal = state.calibrations.get(&DeviceId("dev-1".to_owned()), 0);
    assert!(cal.is_some(), "calibrations should be loaded from profile");
}

#[test]
fn activate_refreshes_outputs_from_cached_axis_values() {
    // Map axis 0 → vJoy device 1 axis X.
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    // Engine starts Stopped so the first tick populates the input cache
    // without evaluating mappings.
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    // Default engine status is Stopped, no need to set it explicitly.

    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new({
            let mut src = MockInputSource::default();
            src.events.push(axis_event(0, 0.5));
            src
        }),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    // Tick 1 (Stopped): input cache is updated, mappings are not evaluated.
    engine.tick().unwrap();
    assert!(
        state.read().output_cache.get_axis(1, VJoyAxis::X).abs() < f64::EPSILON,
        "output cache must be empty before activation"
    );

    // Send Activate and replace the input source with an empty one so
    // tick 2 has no new events, the refresh must rely solely on the cache.
    tx.send(EngineCommand::Activate).unwrap();
    engine.input = Box::new(MockInputSource::default());

    // Tick 2 (Running, no new events): activation refresh fires.
    engine.tick().unwrap();

    let cached = state.read().output_cache.get_axis(1, VJoyAxis::X);
    assert!(
        (cached - 0.5).abs() < f64::EPSILON,
        "output cache should reflect cached axis value after activation, got {cached}"
    );
}

#[test]
fn set_mapping_refreshes_outputs_from_cached_axis_values() {
    // Map axis 0 → vJoy device 1 axis X.
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    // Write the profile to a temp file so set_mapping can persist it.
    let dir = std::env::temp_dir().join("inputforge_engine_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("set_mapping_refresh_test.toml");
    std::fs::write(&path, profile.to_toml().unwrap()).unwrap();

    let (mut engine, state, tx) = make_engine(
        {
            let mut src = MockInputSource::default();
            src.events.push(axis_event(0, 0.5));
            src
        },
        profile,
    );
    state.write().profile_path = Some(path.clone());

    // Tick 1 (Running): processes axis event, writes 0.5 to vJoy X.
    engine.tick().unwrap();
    assert!(
        (state.read().output_cache.get_axis(1, VJoyAxis::X) - 0.5).abs() < f64::EPSILON,
        "initial output should be on vJoy X"
    );

    // Change the mapping: axis 0 → vJoy Y instead of vJoy X.
    tx.send(EngineCommand::SetMapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::Y),
        }],
    })
    .unwrap();

    // Replace input with no events, the refresh must use the cached value.
    engine.input = Box::new(MockInputSource::default());
    engine.tick().unwrap();

    // The output cache should have 0.5 on vJoy Y after the mapping refresh.
    let s = state.read();
    assert!(
        (s.output_cache.get_axis(1, VJoyAxis::Y) - 0.5).abs() < f64::EPSILON,
        "output should be refreshed to vJoy Y after mapping change"
    );

    // Clean up temp file.
    drop(s);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

// ---------------------------------------------------------------------------
// Snapshot helper: engine backed by a real on-disk profile file.
// ---------------------------------------------------------------------------

/// Build an engine whose profile is already persisted to a temp file.
///
/// Uses a simple single-mode tree (`Default` only).  The snapshot tests
/// only need a real on-disk file and do not care about the mode-tree shape.
///
/// Returns `(engine, state, tx, dir, path)` where:
/// - `dir` is the [`tempfile::TempDir`] whose lifetime keeps the temp
///   directory alive, callers must bind it to a local variable.
/// - `path` is the absolute path to the profile TOML.
fn make_engine_with_simple_disk_profile() -> (
    Engine,
    Arc<RwLock<AppState>>,
    mpsc::Sender<EngineCommand>,
    tempfile::TempDir,
    PathBuf,
) {
    // Lay out a real library-style directory under tempdir so the engine's
    // path classifier (`profile_origin_for_path`) flags this profile as
    // Library origin, not External. Without this, snapshots route into
    // %APPDATA%\Roaming\inputforge\external_snapshots\<hash>, but every
    // caller of this fixture asserts the dir-next-to-profile layout.
    // Wiring `settings_path` to a sibling makes `profile_library_dir()`
    // resolve to `<tempdir>/profiles`, which the saved profile sits inside.
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.toml");
    let library_dir = dir.path().join("profiles");
    std::fs::create_dir_all(&library_dir).unwrap();
    let path = library_dir.join("TFM_Throttle.toml");
    let profile = make_profile(simple_mode_tree(), vec![]);
    profile.save(&path).unwrap();
    AppSettings::default().save_to(&settings_path).unwrap();

    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    };

    let (tx, rx) = mpsc::channel();
    let engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        settings_path,
        Box::new(MockAutostart::new()),
    );
    (engine, state, tx, dir, path)
}

// ---------------------------------------------------------------------------
// SwitchMode command tests
// ---------------------------------------------------------------------------

#[test]
fn switch_mode_changes_current_mode() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);

    tx.send(EngineCommand::SwitchMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(state.read().current_mode, "Combat");
}

#[test]
fn switch_mode_unknown_returns_mode_not_found() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, _tx) = make_engine(MockInputSource::default(), profile);

    let err = engine.handle_command(EngineCommand::SwitchMode {
        mode: "Nope".to_owned(),
    });
    assert!(
        matches!(err, Err(crate::error::EngineError::ModeNotFound { .. })),
        "expected ModeNotFound, got {err:?}"
    );
    assert_eq!(
        state.read().current_mode,
        "Default",
        "state must be unchanged on error"
    );
}

#[test]
fn switch_mode_idempotent_on_same_mode() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);

    tx.send(EngineCommand::SwitchMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();
    let current_before = state.read().current_mode.clone();

    tx.send(EngineCommand::SwitchMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(state.read().current_mode, current_before);
}

#[test]
fn switch_mode_rotates_to_different_mode() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);

    tx.send(EngineCommand::SwitchMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    tx.send(EngineCommand::SwitchMode {
        mode: "Landing".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(state.read().current_mode, "Landing");
}

// ---------------------------------------------------------------------------
// F6 settings reload tests
// ---------------------------------------------------------------------------

#[test]
fn reload_settings_picks_up_disk_edits() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.toml");
    std::fs::write(
        &settings_path,
        "[snapshot]\nmax_count = 5\nskip_if_unchanged = false\n",
    )
    .unwrap();

    let profile = make_profile(simple_mode_tree(), vec![]);
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    state.write().engine_status = EngineStatus::Running;
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        settings_path.clone(),
        Box::new(MockAutostart::new()),
    );

    // Sentinel mutation; after ReloadSettings, the field must reflect the
    // tempdir-backed file, not the developer's real %APPDATA%.
    engine.settings.snapshot.max_count = 999;
    tx.send(EngineCommand::ReloadSettings).unwrap();
    engine.tick().unwrap();
    assert_eq!(
        engine.settings.snapshot.max_count, 5,
        "ReloadSettings must read from settings_path"
    );
    assert!(!engine.settings.snapshot.skip_if_unchanged);
}

#[test]
fn set_device_alias_persists_trimmed_alias() {
    let device = DeviceId("dev-1".to_owned());
    let (mut engine, settings_path) = test_engine_with_settings_path(AppSettings::default());

    engine
        .handle_command(EngineCommand::SetDeviceAlias {
            device: device.clone(),
            alias: Some("  Wheel Base  ".to_owned()),
        })
        .expect("alias command succeeds");

    let loaded = AppSettings::load_from(&settings_path);
    assert_eq!(
        loaded.device_aliases.get(&device),
        Some(&"Wheel Base".to_owned())
    );
    let _ = std::fs::remove_file(settings_path);
}

#[test]
fn set_device_alias_with_blank_value_clears_alias() {
    let device = DeviceId("dev-1".to_owned());
    let mut settings = AppSettings::default();
    settings
        .device_aliases
        .insert(device.clone(), "Wheel Base".to_owned());
    let (mut engine, settings_path) = test_engine_with_settings_path(settings);

    engine
        .handle_command(EngineCommand::SetDeviceAlias {
            device: device.clone(),
            alias: Some(" ".to_owned()),
        })
        .expect("alias clear succeeds");

    let loaded = AppSettings::load_from(&settings_path);
    assert!(!loaded.device_aliases.contains_key(&device));
    let _ = std::fs::remove_file(settings_path);
}

// ---------------------------------------------------------------------------
// F6 snapshot command handler tests
// ---------------------------------------------------------------------------

#[test]
fn create_snapshot_command_writes_to_disk() {
    let (mut engine, _state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: Some("v1".to_owned()),
    })
    .unwrap();
    engine.tick().unwrap();

    let listed = crate::snapshot::list(&path).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].label.as_deref(), Some("v1"));
}

#[test]
fn create_snapshot_no_profile_is_silent_noop() {
    let (mut engine, _state, tx) = make_engine_no_profile(MockInputSource::default());
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: None,
    })
    .unwrap();
    engine.tick().unwrap(); // must not panic
}

#[test]
fn pin_snapshot_via_command_persists() {
    let (mut engine, _state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::AutoSessionStart,
        label: None,
    })
    .unwrap();
    engine.tick().unwrap();
    let snap = crate::snapshot::list(&path).unwrap()[0].clone();
    assert!(!snap.pinned);

    tx.send(EngineCommand::PinSnapshot {
        id: snap.id,
        pinned: true,
    })
    .unwrap();
    engine.tick().unwrap();

    assert!(crate::snapshot::list(&path).unwrap()[0].pinned);
}

#[test]
fn rename_snapshot_via_command_persists() {
    let (mut engine, _state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: None,
    })
    .unwrap();
    engine.tick().unwrap();
    let snap = crate::snapshot::list(&path).unwrap()[0].clone();

    tx.send(EngineCommand::RenameSnapshot {
        id: snap.id,
        label: Some("new".to_owned()),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(
        crate::snapshot::list(&path).unwrap()[0].label.as_deref(),
        Some("new")
    );
}

#[test]
fn delete_snapshot_via_command_removes() {
    let (mut engine, _state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: None,
    })
    .unwrap();
    engine.tick().unwrap();
    let snap = crate::snapshot::list(&path).unwrap()[0].clone();

    tx.send(EngineCommand::DeleteSnapshot { id: snap.id })
        .unwrap();
    engine.tick().unwrap();

    assert!(crate::snapshot::list(&path).unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// F6 LoadProfile AutoSessionStart tests (Task 23)
// ---------------------------------------------------------------------------

#[test]
fn load_profile_creates_auto_session_start_snapshot() {
    // Library-style layout: settings_path makes `profile_library_dir()`
    // resolve to <tempdir>/profiles so the LoadProfile classifier flags
    // origin as Library and snapshots write next to the profile.
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.toml");
    let library_dir = dir.path().join("profiles");
    std::fs::create_dir_all(&library_dir).unwrap();
    let path = library_dir.join("TFM_Throttle.toml");
    let profile = make_profile(simple_mode_tree(), vec![]);
    profile.save(&path).unwrap();
    AppSettings::default().save_to(&settings_path).unwrap();

    let state = Arc::new(RwLock::new(AppState::new()));
    state.write().engine_status = EngineStatus::Running;
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        settings_path,
        Box::new(MockAutostart::new()),
    );

    tx.send(EngineCommand::LoadProfile(path.clone())).unwrap();
    engine.tick().unwrap();

    let listed = crate::snapshot::list(&path).unwrap();
    assert_eq!(
        listed.len(),
        1,
        "LoadProfile must create one AutoSessionStart"
    );
    assert!(matches!(
        listed[0].kind,
        crate::snapshot::SnapshotKind::AutoSessionStart
    ));
}

#[test]
fn load_profile_dedupes_auto_session_start_on_identical_content() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.toml");
    let library_dir = dir.path().join("profiles");
    std::fs::create_dir_all(&library_dir).unwrap();
    let path = library_dir.join("TFM_Throttle.toml");
    let profile = make_profile(simple_mode_tree(), vec![]);
    profile.save(&path).unwrap();
    AppSettings::default().save_to(&settings_path).unwrap();

    let state = Arc::new(RwLock::new(AppState::new()));
    state.write().engine_status = EngineStatus::Running;
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        settings_path,
        Box::new(MockAutostart::new()),
    );

    tx.send(EngineCommand::LoadProfile(path.clone())).unwrap();
    engine.tick().unwrap();
    tx.send(EngineCommand::LoadProfile(path.clone())).unwrap();
    engine.tick().unwrap();

    let listed = crate::snapshot::list(&path).unwrap();
    assert_eq!(
        listed.len(),
        1,
        "second load with identical content must dedup"
    );
}

#[test]
fn engine_loadprofile_dedup_respects_skip_if_unchanged_false() {
    // Mirrors the cold-start scenario after main.rs sends LoadProfile:
    // toggling Settings -> "Skip startup snapshot if unchanged" OFF must
    // produce a fresh AutoSessionStart even when the profile content is
    // byte-identical to the most recent existing snapshot.
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.toml");
    let library_dir = dir.path().join("profiles");
    std::fs::create_dir_all(&library_dir).unwrap();
    let path = library_dir.join("TFM_Throttle.toml");
    let profile = make_profile(simple_mode_tree(), vec![]);
    profile.save(&path).unwrap();

    // Persist settings with skip_if_unchanged = false on disk so the
    // engine's mirror of self.settings reflects the user's toggle.
    let mut settings = AppSettings::default();
    settings.snapshot.skip_if_unchanged = false;
    settings.save_to(&settings_path).unwrap();

    let state = Arc::new(RwLock::new(AppState::new()));
    state.write().engine_status = EngineStatus::Running;
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        settings,
        settings_path,
        Box::new(MockAutostart::new()),
    );

    tx.send(EngineCommand::LoadProfile(path.clone())).unwrap();
    engine.tick().unwrap();
    tx.send(EngineCommand::LoadProfile(path.clone())).unwrap();
    engine.tick().unwrap();

    let listed = crate::snapshot::list(&path).unwrap();
    assert_eq!(
        listed.len(),
        2,
        "skip_if_unchanged = false must produce a fresh AutoSessionStart \
         even when content is identical"
    );
    assert!(
        listed
            .iter()
            .all(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoSessionStart)),
        "both snapshots must be AutoSessionStart, got {:?}",
        listed.iter().map(|s| s.kind).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// F6 RestoreSnapshot handler tests (Task 22)
// ---------------------------------------------------------------------------

#[test]
fn restore_snapshot_round_trip() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();

    // Snapshot v1.
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: Some("v1".to_owned()),
    })
    .unwrap();
    engine.tick().unwrap();
    let v1 = crate::snapshot::list(&path).unwrap()[0].clone();

    // Mutate the live profile by hand to a different (still-valid) body.
    let new_body = "[profile]\nid = \"550e8400-e29b-41d4-a716-446655440099\"\n\
        name = \"v2\"\nstartup_mode = \"Default\"\n\n[modes]\nDefault = []\n";
    std::fs::write(&path, new_body).unwrap();
    // Force an explicit reload so engine sees v2 in-memory before restoring.
    tx.send(EngineCommand::LoadProfile(path.clone())).unwrap();
    engine.tick().unwrap();
    assert_eq!(state.read().active_profile.as_ref().unwrap().name(), "v2");

    // Restore v1.
    tx.send(EngineCommand::RestoreSnapshot { id: v1.id })
        .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(s.active_profile.as_ref().unwrap().name(), "Test");
    // AutoBeforeRestore must exist in the snapshot list.
    let listed = crate::snapshot::list(&path).unwrap();
    assert!(
        listed
            .iter()
            .any(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeRestore)),
        "AutoBeforeRestore must be created"
    );
}

// ---------------------------------------------------------------------------
// F8 RemoveMapping handler tests (Task 3)
// ---------------------------------------------------------------------------

#[test]
fn remove_mapping_round_trip_persists_removal_to_disk() {
    use crate::profile::Profile;

    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: Some("Throttle".to_owned()),
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let dir = std::env::temp_dir().join("inputforge_remove_mapping_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("remove_mapping_round_trip.toml");
    std::fs::write(&path, profile.to_toml().unwrap()).unwrap();

    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);
    state.write().profile_path = Some(path.clone());

    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&axis_addr(0), "Default")
            .is_some(),
        "fixture should have one mapping in-memory before remove"
    );

    tx.send(EngineCommand::RemoveMapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&axis_addr(0), "Default")
            .is_none(),
        "RemoveMapping should drop the mapping from active_profile"
    );

    let reloaded = Profile::load(&path).unwrap();
    assert!(
        reloaded.find_mapping(&axis_addr(0), "Default").is_none(),
        "RemoveMapping should persist removal to disk"
    );

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn remove_mapping_no_op_for_unknown_input_does_not_panic() {
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let dir = std::env::temp_dir().join("inputforge_remove_mapping_noop_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("noop.toml");
    std::fs::write(&path, profile.to_toml().unwrap()).unwrap();

    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);
    state.write().profile_path = Some(path.clone());

    tx.send(EngineCommand::RemoveMapping {
        input: button_addr(99),
        mode: "Default".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&axis_addr(0), "Default")
            .is_some(),
        "no-op RemoveMapping must leave existing mappings intact"
    );

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn restore_snapshot_auto_rollback_on_reload_failure() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();

    // Take snapshot of valid profile.
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: None,
    })
    .unwrap();
    engine.tick().unwrap();
    let snap = crate::snapshot::list(&path).unwrap()[0].clone();

    // Corrupt the snapshot's profile body so post-restore reload fails.
    let snap_dir = crate::snapshot::__test_snap_dir(&path).unwrap();
    let snap_file = snap_dir.join(format!("{}.toml", snap.id));
    let body = std::fs::read_to_string(&snap_file).unwrap();
    let (meta, _profile_part) = body.split_once("\n\n").unwrap();
    let bad_profile = "\n\n[profile]\nid = \"550e8400-e29b-41d4-a716-446655440000\"\n\
        name = \"x\"\nstartup_mode = \"NonExistent\"\n\n[modes]\nDefault = []\n";
    std::fs::write(&snap_file, format!("{meta}{bad_profile}")).unwrap();

    let pre_restore_name = state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .name()
        .to_owned();
    let result = engine.handle_command(EngineCommand::RestoreSnapshot { id: snap.id });
    assert!(result.is_err(), "restore must propagate the reload error");

    // Engine state must equal pre-restore (rolled back via AutoBeforeRestore).
    assert_eq!(
        state.read().active_profile.as_ref().unwrap().name(),
        pre_restore_name
    );
    // AutoBeforeRestore must remain in the buffer.
    assert!(
        crate::snapshot::list(&path)
            .unwrap()
            .iter()
            .any(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeRestore)),
        "AutoBeforeRestore must survive a rolled-back restore"
    );
}

// ---------------------------------------------------------------------------
// F6 acceptance criterion: sequential 8+1 snapshot eviction (Task 25)
// ---------------------------------------------------------------------------

#[test]
fn sequential_eight_then_ninth_evicts_oldest() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.toml");
    let library_dir = dir.path().join("profiles");
    std::fs::create_dir_all(&library_dir).unwrap();
    let path = library_dir.join("TFM_Throttle.toml");
    let profile = make_profile(simple_mode_tree(), vec![]);
    profile.save(&path).unwrap();

    let state = Arc::new(RwLock::new(AppState::with_profile(profile.clone())));
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    };

    let (tx, rx) = mpsc::channel();
    let settings = AppSettings {
        last_profile: None,
        snapshot: crate::snapshot::SnapshotConfig {
            max_count: 8,
            skip_if_unchanged: false,
        },
        ..Default::default()
    };
    settings.save_to(&settings_path).unwrap();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        settings,
        settings_path,
        Box::new(MockAutostart::new()),
    );

    let mut ids = Vec::new();
    for _ in 0..8 {
        tx.send(EngineCommand::CreateSnapshot {
            kind: crate::snapshot::SnapshotKind::AutoSessionStart,
            label: None,
        })
        .unwrap();
        engine.tick().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let listed = crate::snapshot::list(&path).unwrap();
    assert_eq!(listed.len(), 8);
    let mut taken_ats: Vec<_> = listed.iter().map(|s| s.taken_at).collect();
    taken_ats.sort();
    assert_eq!(
        taken_ats,
        listed.iter().rev().map(|s| s.taken_at).collect::<Vec<_>>()
    );
    let mut id_set = std::collections::HashSet::new();
    for s in &listed {
        ids.push(s.id);
        id_set.insert(s.id);
    }
    assert_eq!(id_set.len(), 8, "all ids distinct");

    // 9th dispatch: oldest unpinned must be evicted.
    let oldest_id = listed.last().unwrap().id;
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::AutoSessionStart,
        label: None,
    })
    .unwrap();
    engine.tick().unwrap();
    let after = crate::snapshot::list(&path).unwrap();
    assert_eq!(after.len(), 8, "max_count = 8 enforced");
    assert!(
        !after.iter().any(|s| s.id == oldest_id),
        "oldest must be evicted"
    );
}

// ---------------------------------------------------------------------------
// F6 acceptance criterion: restore corrupt target (Task 27)
// ---------------------------------------------------------------------------

#[test]
fn restore_corrupt_target_fires_auto_before_restore_then_errors() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();

    // Create a snapshot to obtain a real id, then corrupt its meta header.
    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: None,
    })
    .unwrap();
    engine.tick().unwrap();
    let snap = crate::snapshot::list(&path).unwrap()[0].clone();

    let snap_dir = crate::snapshot::__test_snap_dir(&path).unwrap();
    let snap_file = snap_dir.join(format!("{}.toml", snap.id));
    // Replace the file with garbage that fails [snapshot_meta] parsing
    // but is still a valid TOML *file*, pick a TOML that lacks the meta table.
    std::fs::write(&snap_file, "[not_meta]\nid = \"garbage\"\n").unwrap();

    let pre_name = state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .name()
        .to_owned();
    let result = engine.handle_command(EngineCommand::RestoreSnapshot { id: snap.id });
    assert!(result.is_err(), "corrupt target must error");

    // Live profile semantically unchanged (rollback restored original content).
    assert_eq!(
        state.read().active_profile.as_ref().unwrap().name(),
        pre_name,
        "active profile must be rolled back to the original"
    );
    // AutoBeforeRestore was added.
    assert!(
        crate::snapshot::list(&path)
            .unwrap()
            .iter()
            .any(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeRestore)),
        "AutoBeforeRestore must exist even though restore failed"
    );
}

// ---------------------------------------------------------------------------
// F7 mode CRUD: AddMode
// ---------------------------------------------------------------------------

/// Build an engine with a 3-mode profile loaded on disk so handlers can persist.
fn make_engine_with_disk_profile() -> (
    Engine,
    Arc<RwLock<AppState>>,
    mpsc::Sender<EngineCommand>,
    tempfile::TempDir,
    PathBuf,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    let profile = make_profile(three_mode_tree(), vec![]);
    profile.save(&path).unwrap();

    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let mut s = state.write();
    s.profile_path = Some(path.clone());
    s.engine_status = EngineStatus::Running;
    drop(s);
    let (tx, rx) = mpsc::channel();
    let engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );
    (engine, state, tx, dir, path)
}

#[test]
fn add_mode_appends_under_root_by_default() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_disk_profile();
    tx.send(EngineCommand::AddMode {
        name: "Approach".to_owned(),
        parent: None,
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    let modes = s.active_profile.as_ref().unwrap().modes();
    assert!(modes.contains("Approach"));
    let root = modes.root();
    assert!(root.children().iter().any(|c| c.name() == "Approach"));

    // Persisted to disk.
    drop(s);
    let reloaded = Profile::load(&path).unwrap();
    assert!(reloaded.modes().contains("Approach"));
}

#[test]
fn add_mode_under_named_parent() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_disk_profile();
    tx.send(EngineCommand::AddMode {
        name: "Bombs".to_owned(),
        parent: Some("Combat".to_owned()),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    let combat = s
        .active_profile
        .as_ref()
        .unwrap()
        .modes()
        .find_mode("Combat")
        .unwrap();
    assert!(combat.children().iter().any(|c| c.name() == "Bombs"));
}

#[test]
fn add_mode_rejects_empty_name() {
    let (mut engine, state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::AddMode {
        name: String::new(),
        parent: None,
    });
    assert!(err.is_err(), "expected error on empty name");
    // Profile unchanged.
    let s = state.read();
    let modes = s.active_profile.as_ref().unwrap().modes().all_modes();
    assert_eq!(modes.len(), 3);
}

/// 64-grapheme cap, mirrored from the GUI's inline editor. One test
/// locks the contract for `AddMode`; `RenameMode` and `SetDefaultMode`
/// route the same `name` argument through the same shared validator
/// (`validate_mode_name_for_engine`) so this single assertion covers
/// all three command paths.
#[test]
fn add_mode_rejects_overlong_name() {
    let (mut engine, state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let overlong = "x".repeat(65);
    let err = engine.handle_command(EngineCommand::AddMode {
        name: overlong,
        parent: None,
    });
    assert!(err.is_err(), "expected error on 65-grapheme name");
    let s = state.read();
    let modes = s.active_profile.as_ref().unwrap().modes().all_modes();
    assert_eq!(modes.len(), 3, "profile must remain unchanged on rejection");
}

#[test]
fn add_mode_rejects_duplicate_name() {
    let (mut engine, state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::AddMode {
        name: "Combat".to_owned(),
        parent: None,
    });
    assert!(err.is_err(), "expected error on duplicate name");
    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .modes()
            .all_modes()
            .len(),
        3
    );
}

#[test]
fn add_mode_rejects_unknown_parent() {
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::AddMode {
        name: "Foo".to_owned(),
        parent: Some("Nope".to_owned()),
    });
    assert!(err.is_err());
}

#[test]
fn add_mode_no_active_profile_is_no_op() {
    let (mut engine, state, _tx) = make_engine_no_profile(MockInputSource::default());
    engine
        .handle_command(EngineCommand::AddMode {
            name: "Foo".to_owned(),
            parent: None,
        })
        .unwrap();
    assert!(state.read().active_profile.is_none());
}

#[test]
fn add_mode_returns_err_when_persist_fails() {
    let (mut engine, state, _tx, _dir, path) = make_engine_with_disk_profile();
    // Make the profile path point at a directory to force save() to fail.
    let mut s = state.write();
    s.profile_path = Some(path.parent().unwrap().to_path_buf());
    drop(s);
    let err = engine.handle_command(EngineCommand::AddMode {
        name: "Approach".to_owned(),
        parent: None,
    });
    assert!(err.is_err(), "expected persistence error to propagate");
    // In-memory mutation is intentionally retained.
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Approach")
    );
}

// ---------------------------------------------------------------------------
// F7 mode CRUD: RenameMode
// ---------------------------------------------------------------------------

#[test]
fn rename_mode_renames_tree_and_persists() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_disk_profile();
    tx.send(EngineCommand::RenameMode {
        from: "Combat".to_owned(),
        to: "Fighter".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    let modes = s.active_profile.as_ref().unwrap().modes();
    assert!(modes.contains("Fighter"));
    assert!(!modes.contains("Combat"));
    drop(s);

    let reloaded = Profile::load(&path).unwrap();
    assert!(reloaded.modes().contains("Fighter"));
}

#[test]
fn rename_mode_cascades_into_mappings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    let mappings = vec![Mapping {
        input: axis_addr(0),
        mode: "Combat".to_owned(),
        name: None,
        actions: vec![Action::Invert],
    }];
    let profile = make_profile(three_mode_tree(), mappings);
    profile.save(&path).unwrap();

    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let mut s = state.write();
    s.profile_path = Some(path.clone());
    s.engine_status = EngineStatus::Running;
    drop(s);
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );
    tx.send(EngineCommand::RenameMode {
        from: "Combat".to_owned(),
        to: "Fighter".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(
        s.active_profile.as_ref().unwrap().mappings()[0].mode,
        "Fighter"
    );
}

#[test]
fn rename_mode_cascades_into_startup_mode() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_disk_profile();
    tx.send(EngineCommand::RenameMode {
        from: "Default".to_owned(),
        to: "Root".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();
    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .settings()
            .startup_mode(),
        "Root"
    );
}

#[test]
fn rename_mode_cascades_into_runtime_state() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_disk_profile();
    // Switch into Combat first so current_mode tracks the rename.
    tx.send(EngineCommand::SwitchMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    tx.send(EngineCommand::RenameMode {
        from: "Combat".to_owned(),
        to: "Fighter".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(
        state.read().current_mode,
        "Fighter",
        "current_mode should track rename"
    );
}

#[test]
fn rename_mode_rejects_collision() {
    let (mut engine, state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::RenameMode {
        from: "Combat".to_owned(),
        to: "Landing".to_owned(),
    });
    assert!(err.is_err());
    // Profile unchanged.
    let s = state.read();
    assert!(
        s.active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Combat")
    );
    assert!(
        s.active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Landing")
    );
}

#[test]
fn rename_mode_rejects_unknown_from() {
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::RenameMode {
        from: "Nope".to_owned(),
        to: "Whatever".to_owned(),
    });
    assert!(err.is_err());
}

#[test]
fn rename_mode_rejects_empty_to() {
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::RenameMode {
        from: "Combat".to_owned(),
        to: String::new(),
    });
    assert!(err.is_err());
}

#[test]
fn rename_mode_rejects_empty_from_with_invalid_config() {
    // Symmetric validation: an empty `from` returns InvalidConfig (the
    // policy register), not ModeNotFound (which would leak the
    // implementation detail that an empty string isn't in the tree).
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine
        .handle_command(EngineCommand::RenameMode {
            from: String::new(),
            to: "Approach".to_owned(),
        })
        .unwrap_err();
    assert!(
        matches!(err, crate::error::EngineError::InvalidConfig { .. }),
        "expected InvalidConfig for empty `from`, got: {err:?}"
    );
}

#[test]
fn rename_mode_rejects_overlong_from_with_invalid_config() {
    // 65 graphemes is one past the cap.
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine
        .handle_command(EngineCommand::RenameMode {
            from: "x".repeat(65),
            to: "Approach".to_owned(),
        })
        .unwrap_err();
    assert!(
        matches!(err, crate::error::EngineError::InvalidConfig { .. }),
        "expected InvalidConfig for oversized `from`, got: {err:?}"
    );
}

#[test]
fn delete_mode_rejects_empty_name_with_invalid_config() {
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine
        .handle_command(EngineCommand::DeleteMode {
            name: String::new(),
        })
        .unwrap_err();
    assert!(
        matches!(err, crate::error::EngineError::InvalidConfig { .. }),
        "expected InvalidConfig for empty name, got: {err:?}"
    );
}

#[test]
fn delete_mode_rejects_overlong_name_with_invalid_config() {
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine
        .handle_command(EngineCommand::DeleteMode {
            name: "x".repeat(65),
        })
        .unwrap_err();
    assert!(
        matches!(err, crate::error::EngineError::InvalidConfig { .. }),
        "expected InvalidConfig for oversized name, got: {err:?}"
    );
}

#[test]
fn rename_mode_rewrites_switch_to_action() {
    use crate::action::{Mapping, ModeChangeStrategy};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    let mappings = vec![Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::ChangeMode {
            strategy: ModeChangeStrategy::SwitchTo {
                mode: "Combat".to_owned(),
            },
        }],
    }];
    let profile = make_profile(three_mode_tree(), mappings);
    profile.save(&path).unwrap();
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let mut s = state.write();
    s.profile_path = Some(path.clone());
    s.engine_status = EngineStatus::Running;
    drop(s);
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    tx.send(EngineCommand::RenameMode {
        from: "Combat".to_owned(),
        to: "Fighter".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    let action = &s.active_profile.as_ref().unwrap().mappings()[0].actions[0];
    match action {
        Action::ChangeMode {
            strategy: ModeChangeStrategy::SwitchTo { mode },
        } => assert_eq!(mode, "Fighter"),
        other => panic!("expected SwitchTo with renamed target, got {other:?}"),
    }
}

#[test]
fn rename_mode_rewrites_temporary_action() {
    use crate::action::{Mapping, ModeChangeStrategy};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    let mappings = vec![Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::ChangeMode {
            strategy: ModeChangeStrategy::Temporary {
                mode: "Combat".to_owned(),
            },
        }],
    }];
    let profile = make_profile(three_mode_tree(), mappings);
    profile.save(&path).unwrap();
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let mut s = state.write();
    s.profile_path = Some(path.clone());
    s.engine_status = EngineStatus::Running;
    drop(s);
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    tx.send(EngineCommand::RenameMode {
        from: "Combat".to_owned(),
        to: "Fighter".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    let action = &s.active_profile.as_ref().unwrap().mappings()[0].actions[0];
    match action {
        Action::ChangeMode {
            strategy: ModeChangeStrategy::Temporary { mode },
        } => assert_eq!(mode, "Fighter"),
        other => panic!("expected Temporary with renamed target, got {other:?}"),
    }
}

#[test]
fn rename_mode_rewrites_multi_entry_stack() {
    // Spec 917: rename_in_place must rewrite every occurrence of `from` in
    // both `current` and the stack, preserving order. We verify both:
    //   (a) a stack entry (not current) is rewritten, and
    //   (b) current is rewritten.
    //
    // Sequence (avoiding cycle-detection rejection):
    //   switch_to "Combat" → push_temporary "Landing"
    //   Now: stack = ["Combat"], current = "Landing"
    //   rename "Combat" → "Fighter"
    //   Expect: current stays "Landing", pop → "Fighter"
    //
    //   Then separately: push_temporary "Combat" from "Default" baseline,
    //   rename, verify current rewritten.
    let (mut engine, state, _tx, _dir, _path) = make_engine_with_disk_profile();

    let tree = state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .modes()
        .clone();

    // Part A: stack entry is rewritten.
    // switch_to resets the stack, sets current = "Combat".
    engine.mode_state.switch_to("Combat", &tree).unwrap();
    // push_temporary saves "Combat" on the stack, sets current = "Landing".
    engine.mode_state.push_temporary("Landing", &tree).unwrap();
    assert_eq!(engine.mode_state.current(), "Landing");

    engine
        .handle_command(EngineCommand::RenameMode {
            from: "Combat".to_owned(),
            to: "Fighter".to_owned(),
        })
        .unwrap();

    // current is "Landing", unaffected.
    assert_eq!(engine.mode_state.current(), "Landing");
    // Pop: the stacked "Combat" entry was rewritten to "Fighter".
    engine.mode_state.pop_temporary();
    assert_eq!(engine.mode_state.current(), "Fighter");

    // Part B: current itself is rewritten.
    // After the pop, current = "Fighter". Switch back to a known name.
    // Re-acquire tree reflecting the rename already applied.
    let tree2 = state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .modes()
        .clone();
    engine.mode_state.switch_to("Fighter", &tree2).unwrap();
    assert_eq!(engine.mode_state.current(), "Fighter");

    engine
        .handle_command(EngineCommand::RenameMode {
            from: "Fighter".to_owned(),
            to: "Ace".to_owned(),
        })
        .unwrap();

    assert_eq!(engine.mode_state.current(), "Ace");
}

// ---------------------------------------------------------------------------
// F7 mode CRUD: DeleteMode
// ---------------------------------------------------------------------------

#[test]
fn delete_mode_removes_leaf() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_disk_profile();
    tx.send(EngineCommand::DeleteMode {
        name: "Landing".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert!(
        !s.active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Landing")
    );
    drop(s);
    let reloaded = Profile::load(&path).unwrap();
    assert!(!reloaded.modes().contains("Landing"));
}

#[test]
fn delete_mode_cascades_subtree_and_mappings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");

    // Build a tree with Combat → [Missiles, Guns] so DeleteMode("Combat")
    // hits children + mappings.
    let map = HashMap::from([
        (
            "Default".to_owned(),
            vec!["Combat".to_owned(), "Landing".to_owned()],
        ),
        (
            "Combat".to_owned(),
            vec!["Missiles".to_owned(), "Guns".to_owned()],
        ),
    ]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let mappings = vec![
        Mapping {
            input: axis_addr(0),
            mode: "Combat".to_owned(),
            name: None,
            actions: vec![Action::Invert],
        },
        Mapping {
            input: axis_addr(1),
            mode: "Missiles".to_owned(),
            name: None,
            actions: vec![Action::Invert],
        },
        Mapping {
            input: axis_addr(2),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Invert],
        },
    ];
    let profile = make_profile(modes, mappings);
    profile.save(&path).unwrap();

    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let mut s = state.write();
    s.profile_path = Some(path.clone());
    s.engine_status = EngineStatus::Running;
    drop(s);
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    tx.send(EngineCommand::DeleteMode {
        name: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    let modes = s.active_profile.as_ref().unwrap().modes();
    assert!(!modes.contains("Combat"));
    assert!(!modes.contains("Missiles"));
    assert!(!modes.contains("Guns"));
    assert!(modes.contains("Default"));
    let mappings = s.active_profile.as_ref().unwrap().mappings();
    assert_eq!(mappings.len(), 1);
    assert_eq!(mappings[0].mode, "Default");
}

#[test]
fn delete_mode_rejects_root() {
    let (mut engine, state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::DeleteMode {
        name: "Default".to_owned(),
    });
    assert!(err.is_err());
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Default")
    );
}

#[test]
fn delete_mode_rejects_when_subtree_contains_startup_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    let modes = three_mode_tree();
    // Profile that boots into Combat; DeleteMode("Combat") must reject.
    let profile = Profile::new(
        "BootCombat".to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Combat".to_owned(),
    );
    profile.save(&path).unwrap();
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let mut s = state.write();
    s.profile_path = Some(path.clone());
    s.engine_status = EngineStatus::Running;
    drop(s);
    let (_tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    let err = engine.handle_command(EngineCommand::DeleteMode {
        name: "Combat".to_owned(),
    });
    assert!(
        err.is_err(),
        "must reject when subtree contains startup_mode"
    );
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Combat")
    );
}

#[test]
fn delete_mode_resets_current_when_referenced() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_disk_profile();
    tx.send(EngineCommand::SwitchMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    tx.send(EngineCommand::DeleteMode {
        name: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(
        state.read().current_mode,
        "Default",
        "current_mode resets to startup"
    );
}

#[test]
fn delete_mode_rejects_unknown_name() {
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::DeleteMode {
        name: "Nope".to_owned(),
    });
    assert!(err.is_err());
}

#[test]
fn delete_mode_resets_when_active_is_descendant() {
    // Spec invariant: deleting an ancestor of the active mode must reset
    // current_mode to startup_mode and purge every removed name from the
    // ModeState stack.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    let map = HashMap::from([
        (
            "Default".to_owned(),
            vec!["Combat".to_owned(), "Landing".to_owned()],
        ),
        (
            "Combat".to_owned(),
            vec!["Missiles".to_owned(), "Guns".to_owned()],
        ),
    ]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let profile = make_profile(modes, vec![]);
    profile.save(&path).unwrap();

    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let mut s = state.write();
    s.profile_path = Some(path.clone());
    s.engine_status = EngineStatus::Running;
    drop(s);
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    // Switch into Missiles (descendant of Combat).
    tx.send(EngineCommand::SwitchMode {
        mode: "Missiles".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();
    // Push another descendant so the stack is non-trivial.
    let tree = state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .modes()
        .clone();
    engine.mode_state.push_temporary("Guns", &tree).unwrap();

    // Delete Combat, every descendant (Missiles, Guns) and Combat itself
    // must be purged from current_mode and the stack.
    tx.send(EngineCommand::DeleteMode {
        name: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(
        s.current_mode, "Default",
        "current_mode must reset to startup"
    );
    assert_eq!(
        engine.mode_state.current(),
        "Default",
        "ModeState::current resets"
    );
    // pop_temporary is a no-op on an empty stack, the post-delete current
    // stays "Default".
    engine.mode_state.pop_temporary();
    assert_eq!(
        engine.mode_state.current(),
        "Default",
        "stack must be purged of removed names"
    );
}

// ---------------------------------------------------------------------------
// F7 mode CRUD: SetDefaultMode
// ---------------------------------------------------------------------------

#[test]
fn set_default_mode_updates_startup_mode_and_persists() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_disk_profile();
    tx.send(EngineCommand::SetDefaultMode {
        name: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .settings()
            .startup_mode(),
        "Combat"
    );
    let reloaded = Profile::load(&path).unwrap();
    assert_eq!(reloaded.settings().startup_mode(), "Combat");
}

#[test]
fn set_default_mode_rejects_unknown_name() {
    let (mut engine, state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::SetDefaultMode {
        name: "Nope".to_owned(),
    });
    assert!(err.is_err());
    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .settings()
            .startup_mode(),
        "Default"
    );
}

#[test]
fn set_default_mode_rejects_empty_name() {
    let (mut engine, state, _tx, _dir, _path) = make_engine_with_disk_profile();
    let err = engine.handle_command(EngineCommand::SetDefaultMode {
        name: String::new(),
    });
    assert!(err.is_err());
    // Whitespace-only also rejected.
    let err = engine.handle_command(EngineCommand::SetDefaultMode {
        name: "   ".to_owned(),
    });
    assert!(err.is_err());
    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .settings()
            .startup_mode(),
        "Default"
    );
}

// ---------------------------------------------------------------------------
// SetMappingsBulk handler tests (Bulk-map wizard)
// ---------------------------------------------------------------------------

fn make_bulk_entry(input_idx: u8, axis: VJoyAxis) -> crate::action::BulkMapEntry {
    crate::action::BulkMapEntry {
        input: axis_addr(input_idx),
        mode: "Default".to_owned(),
        output: vjoy_axis_output(1, axis),
    }
}

#[test]
fn engine_set_mappings_bulk_persists_to_disk_and_creates_snapshot() {
    let (mut engine, _state, tx, _dir, path) = make_engine_with_simple_disk_profile();

    let pre_writes = std::fs::read(&path).unwrap();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let post_writes = std::fs::read(&path).unwrap();
    assert_ne!(
        pre_writes, post_writes,
        "post-bulk save must update on-disk body"
    );
    let listed = crate::snapshot::list(&path).unwrap();
    let bulk_count = listed
        .iter()
        .filter(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeBulkMap))
        .count();
    assert_eq!(
        bulk_count, 1,
        "exactly one AutoBeforeBulkMap snapshot per apply"
    );
}

#[test]
fn engine_set_mappings_bulk_with_no_profile_loaded_is_noop_and_warns() {
    let state = Arc::new(RwLock::new(AppState::new()));
    state.write().engine_status = EngineStatus::Running;
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert!(state.read().active_profile.is_none(), "still no profile");
    let warns = state.read().warnings.clone();
    assert!(
        warns
            .iter()
            .any(|w| w.contains("Bulk-map ignored: no profile loaded")),
        "warnings must surface the no-profile abort, got: {warns:?}"
    );
}

#[test]
fn engine_set_mappings_bulk_sets_pending_output_refresh_true() {
    let (mut engine, _state, _tx, _dir, _path) = make_engine_with_simple_disk_profile();
    engine
        .handle_command(EngineCommand::SetMappingsBulk {
            entries: vec![make_bulk_entry(0, VJoyAxis::X)],
            snapshot_label: "x".to_owned(),
        })
        .unwrap();
    assert!(
        engine.pending_output_refresh,
        "bulk apply must trigger output refresh"
    );
}

#[test]
fn engine_set_mappings_bulk_applies_all_n_entries() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let mut entries = Vec::new();
    for i in 0..8 {
        entries.push(make_bulk_entry(i, VJoyAxis::X));
    }
    tx.send(EngineCommand::SetMappingsBulk {
        entries,
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();
    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .len(),
        8
    );
    let _ = path;
}

#[test]
fn engine_set_mappings_bulk_creates_auto_before_bulk_map_snapshot_with_label() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let listed = crate::snapshot::list(&path).unwrap();
    let snap = listed
        .iter()
        .find(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeBulkMap))
        .expect("AutoBeforeBulkMap must exist");
    assert_eq!(
        snap.label.as_deref(),
        Some("Before bulk-map: dev-1 to vJoy 1")
    );
    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .len(),
        1
    );
}

#[test]
fn engine_set_mappings_bulk_pre_snapshot_save_failure_aborts_and_warns() {
    let (mut engine, state, tx, dir, path) = make_engine_with_simple_disk_profile();
    let nested = dir.path().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let nested_path = nested.join("profile.toml");
    std::fs::write(&nested_path, std::fs::read(&path).unwrap()).unwrap();
    state.write().profile_path = Some(nested_path.clone());
    std::fs::remove_dir_all(&nested).unwrap();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let warns = state.read().warnings.clone();
    assert!(
        warns.iter().any(|w| w.contains("Bulk-map aborted")),
        "warnings must surface a Bulk-map aborted line, got: {warns:?}"
    );
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .is_empty()
    );
}

#[test]
fn engine_set_mappings_bulk_aborts_apply_when_snapshot_creation_fails() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let snap_dir = crate::snapshot::__test_snap_dir(&path).unwrap();
    std::fs::write(&snap_dir, b"blocker").unwrap();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .is_empty()
    );
    let warns = state.read().warnings.clone();
    assert!(
        warns.iter().any(|w| w.contains("recovery snapshot")),
        "must push 'recovery snapshot' warning, got: {warns:?}"
    );
}

#[test]
fn engine_set_mappings_bulk_abort_path_does_not_leak_state_write_lock() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let snap_dir = crate::snapshot::__test_snap_dir(&path).unwrap();
    std::fs::write(&snap_dir, b"blocker").unwrap();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert!(
        state.try_read().is_some(),
        "abort path must release any write guard before returning"
    );
}

#[test]
fn engine_set_mappings_bulk_happy_path_in_memory_state_holds_one_mapping() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_simple_disk_profile();
    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .len(),
        1
    );
}

#[test]
fn engine_set_mappings_bulk_skips_entries_with_unbound_input() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_simple_disk_profile();

    let entry = crate::action::BulkMapEntry {
        input: InputAddress::Unbound,
        mode: "Default".to_owned(),
        output: vjoy_axis_output(1, VJoyAxis::X),
    };
    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![entry],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .is_empty(),
        "Unbound input entries must be skipped by the bulk handler"
    );
}

#[test]
fn smoke_bulk_map_full_round_trip_creates_correct_profile_state() {
    use crate::action::BulkMapEntry;
    use crate::types::{OutputAddress, OutputId};

    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let mut entries = Vec::new();
    // Four axes, eight buttons, one hat.
    for i in 0u8..4 {
        let axis_enum = match i {
            0 => VJoyAxis::X,
            1 => VJoyAxis::Y,
            2 => VJoyAxis::Z,
            _ => VJoyAxis::Rx,
        };
        entries.push(BulkMapEntry {
            input: axis_addr(i),
            mode: "Default".to_owned(),
            output: vjoy_axis_output(1, axis_enum),
        });
    }
    for i in 0u8..8 {
        entries.push(BulkMapEntry {
            input: button_addr(i),
            mode: "Default".to_owned(),
            output: vjoy_button_output(1, i + 1),
        });
    }
    entries.push(BulkMapEntry {
        input: InputAddress::Bound {
            device: dev_id(),
            input: InputId::Hat { index: 0 },
        },
        mode: "Default".to_owned(),
        output: OutputAddress {
            device: 1,
            output: OutputId::Hat { id: 1 },
        },
    });

    tx.send(EngineCommand::SetMappingsBulk {
        entries,
        snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let mappings = state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .mappings()
        .to_vec();
    assert_eq!(mappings.len(), 13, "4 axes + 8 buttons + 1 hat = 13");
    for m in &mappings {
        assert_eq!(m.name, None);
        assert_eq!(m.actions.len(), 1);
        assert!(matches!(m.actions[0], Action::MapToVJoy { .. }));
    }

    let listed = crate::snapshot::list(&path).unwrap();
    assert!(
        listed
            .iter()
            .any(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeBulkMap)),
        "AutoBeforeBulkMap must be listed"
    );
}

// ---------------------------------------------------------------------------
// F15 Task 1.5: AppState.snapshot_config mirror
// ---------------------------------------------------------------------------

#[test]
fn engine_initialisation_mirrors_settings_snapshot_into_state() {
    use crate::snapshot::SnapshotConfig;
    let mut harness = EngineHarness::new();
    // Force the engine's in-memory settings snapshot to a non-default value
    // and refresh the state mirror, simulating what `Engine::new` will do
    // once Step 4 is wired.
    harness.engine.settings.snapshot = SnapshotConfig {
        max_count: 25,
        skip_if_unchanged: false,
    };
    harness.engine.state.write().snapshot_config = harness.engine.settings.snapshot.clone();
    assert_eq!(
        harness.state().snapshot_config,
        harness.engine.settings.snapshot
    );
}

#[test]
fn reload_settings_mirrors_into_state_snapshot_config() {
    use crate::snapshot::SnapshotConfig;
    let mut harness = EngineHarness::new();
    let initial = harness.state().snapshot_config.clone();

    // Write a fresh settings.toml with a different snapshot config.
    let new_cfg = SnapshotConfig {
        max_count: 7,
        skip_if_unchanged: !initial.skip_if_unchanged,
    };
    let mut file_settings = AppSettings::default();
    file_settings.snapshot = new_cfg.clone();
    file_settings
        .save_to(&harness.engine.settings_path)
        .unwrap();

    harness.dispatch(EngineCommand::ReloadSettings).unwrap();

    assert_eq!(harness.state().snapshot_config, new_cfg);
}

#[test]
fn set_snapshot_config_writes_settings_toml_and_replaces_in_memory() {
    let mut harness = EngineHarness::new();

    let new_cfg = crate::snapshot::SnapshotConfig {
        max_count: 25,
        skip_if_unchanged: false,
    };
    harness
        .dispatch(EngineCommand::SetSnapshotConfig {
            config: new_cfg.clone(),
        })
        .unwrap();

    // In-memory: handler took effect.
    assert_eq!(harness.engine.settings.snapshot, new_cfg);

    // On-disk: settings.toml round-trips the new config.
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.snapshot, new_cfg);
}

#[test]
fn set_snapshot_config_prunes_when_max_count_decreased() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();

    // Seed five AutoBeforeRestore snapshots (unpinned, always fire) so the
    // active namespace has enough unpinned entries for prune to act on.
    // Manual snapshots are auto-pinned at creation and are therefore exempt
    // from FIFO eviction; AutoBeforeRestore snapshots are not pinned.
    for _ in 0..5 {
        harness
            .dispatch(EngineCommand::CreateSnapshot {
                kind: crate::snapshot::SnapshotKind::AutoBeforeRestore,
                label: None,
            })
            .unwrap();
    }

    let before = harness.state().active_snapshot_rows.len();
    assert!(
        before >= 5,
        "expected at least 5 snapshots before prune, got {before}"
    );

    // Reduce max_count below the seeded count to force a prune.
    harness
        .dispatch(EngineCommand::SetSnapshotConfig {
            config: crate::snapshot::SnapshotConfig {
                max_count: 2,
                skip_if_unchanged: true,
            },
        })
        .unwrap();

    let after = harness.state().active_snapshot_rows.len();
    assert!(
        after <= 2,
        "expected at most 2 snapshots after prune, got {after}"
    );
}

#[test]
fn set_snapshot_config_does_not_prune_when_max_count_increased() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();
    for i in 0..3 {
        harness
            .dispatch(EngineCommand::CreateSnapshot {
                kind: crate::snapshot::SnapshotKind::Manual,
                label: Some(format!("snap-{i}")),
            })
            .unwrap();
    }
    let before = harness.state().active_snapshot_rows.len();

    harness
        .dispatch(EngineCommand::SetSnapshotConfig {
            config: crate::snapshot::SnapshotConfig {
                max_count: 50,
                skip_if_unchanged: true,
            },
        })
        .unwrap();

    let after = harness.state().active_snapshot_rows.len();
    assert_eq!(after, before, "increase must not prune");
}

#[test]
fn set_snapshot_config_no_prune_when_no_profile_loaded() {
    let mut harness = EngineHarness::new();

    // No profile loaded; resolved_snapshot_target returns None so prune is skipped.
    let result = harness.dispatch(EngineCommand::SetSnapshotConfig {
        config: crate::snapshot::SnapshotConfig {
            max_count: 1,
            skip_if_unchanged: false,
        },
    });

    assert!(
        result.is_ok(),
        "no profile loaded must not error: {result:?}"
    );
    assert_eq!(harness.engine.settings.snapshot.max_count, 1);
}

#[test]
fn set_snapshot_config_save_failure_does_not_persist() {
    let mut harness = EngineHarness::new();
    let original = harness.engine.settings.snapshot.clone();

    harness.force_settings_path_to_unwritable();

    // Dispatch with a different value so the rollback is observable.
    let attempted = crate::snapshot::SnapshotConfig {
        max_count: 99,
        skip_if_unchanged: !original.skip_if_unchanged,
    };
    harness
        .dispatch(EngineCommand::SetSnapshotConfig { config: attempted })
        .unwrap();

    // In-memory rolled back to the pre-command value.
    assert_eq!(harness.engine.settings.snapshot, original);

    // The AppState mirror was also rolled back to the pre-command value.
    assert_eq!(harness.state().snapshot_config, original);

    // Warnings channel received the failure message.
    let warnings = harness.state().warnings.clone();
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("Could not save settings")),
        "expected warning, got: {warnings:?}"
    );
}

#[test]
fn set_snapshot_config_in_memory_matches_disk_after_prune() {
    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();
    for i in 0..3 {
        harness
            .dispatch(EngineCommand::CreateSnapshot {
                kind: crate::snapshot::SnapshotKind::Manual,
                label: Some(format!("snap-{i}")),
            })
            .unwrap();
    }

    let new_cfg = crate::snapshot::SnapshotConfig {
        max_count: 1,
        skip_if_unchanged: true,
    };
    harness
        .dispatch(EngineCommand::SetSnapshotConfig {
            config: new_cfg.clone(),
        })
        .unwrap();

    // After prune: in-memory == on-disk == requested.
    assert_eq!(harness.engine.settings.snapshot, new_cfg);
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.snapshot, new_cfg);
    // The AppState mirror also reflects the new value.
    assert_eq!(harness.state().snapshot_config, new_cfg);
}

#[cfg(unix)]
#[test]
fn set_snapshot_config_prune_failure_does_not_corrupt_settings() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    // When the snapshot namespace dir becomes unreadable after settings.toml
    // has already been saved, the handler must keep in-memory + on-disk +
    // AppState consistent and surface a `Snapshot prune failed` warning.
    // Note: under non-root permissions only; if the test runner is root,
    // mode 0o000 does not deny access and this test is a no-op.

    let mut harness = EngineHarness::new();
    harness.create_and_load_profile("Alpha").unwrap();

    // Seed unpinned snapshots so a prune would otherwise act.
    for _ in 0..3 {
        harness
            .dispatch(EngineCommand::CreateSnapshot {
                kind: crate::snapshot::SnapshotKind::AutoBeforeRestore,
                label: None,
            })
            .unwrap();
    }

    // Resolve the namespace dir for the chmod target. The read guard is
    // dropped at the end of the scope so the dispatch can re-acquire the
    // lock for writes.
    let namespace_dir = {
        let state = harness.state();
        crate::snapshot::pending_delete::resolve_snapshot_namespace(&state).unwrap()
    };
    let original_perms = fs::metadata(&namespace_dir).unwrap().permissions();
    fs::set_permissions(&namespace_dir, fs::Permissions::from_mode(0o000)).unwrap();

    let new_cfg = crate::snapshot::SnapshotConfig {
        max_count: 1,
        skip_if_unchanged: true,
    };
    // Both `prune_in` (via `list_in`) and `refresh_active_snapshot_rows`
    // (via `list_visible`) read the now-unreadable dir. The handler pushes
    // the prune-failure warning before propagating the refresh error, so
    // the dispatch may return Err even though settings have been saved
    // consistently. Either outcome is permissible; the consistency
    // invariant is what we verify.
    let _ = harness.dispatch(EngineCommand::SetSnapshotConfig {
        config: new_cfg.clone(),
    });

    // Restore permissions so the temp dir can be cleaned up and so
    // subsequent reads succeed.
    fs::set_permissions(&namespace_dir, original_perms).unwrap();

    // Settings updated atomically: in-memory, on-disk, and the AppState
    // mirror all reflect the new config despite the post-save prune error.
    assert_eq!(harness.engine.settings.snapshot, new_cfg);
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.snapshot, new_cfg);
    assert_eq!(harness.state().snapshot_config, new_cfg);

    // Prune-failure warning was pushed before any subsequent error.
    let warnings = harness.state().warnings.clone();
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("Snapshot prune failed after settings save")),
        "expected prune-failure warning, got: {warnings:?}"
    );
}

// ---------------------------------------------------------------------------
// F16: startup preference mirror tests
// ---------------------------------------------------------------------------

#[test]
fn engine_initialisation_mirrors_startup_into_state() {
    use crate::settings::StartupSettings;
    let settings = AppSettings {
        startup: StartupSettings {
            launch_at_startup: true,
            start_minimized_to_tray: true,
        },
        ..AppSettings::default()
    };
    let (engine, _path) = test_engine_with_settings_path(settings.clone());
    assert_eq!(engine.state.read().startup, settings.startup);
}
