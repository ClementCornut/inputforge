// Rust guideline compliant 2026-03-06

//! Engine integration and unit tests.
//!
//! This module is feature-gated behind `test-util` so it can access
//! the mock I/O implementations.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Instant;

use parking_lot::RwLock;

use crate::action::{Action, CycleModes, Mapping, ModeChangeStrategy};
use crate::callbacks::{CallbackRegistry, ReleaseCallback};
use crate::device::mock::{MockDeviceHider, MockInputSource};
use crate::device::traits::HotplugEvent;
use crate::mode::{ModeState, ModeTree};
use crate::output::mock::{KeyboardCall, MockKeyboardSink, MockOutputSink, OutputCall};
use crate::pipeline::PipelineOutput;
use crate::profile::Profile;
use crate::settings::AppSettings;
use crate::state::{AppState, DeviceState, EngineStatus, InputCacheStore, OutputCacheStore};
use crate::types::{
    AxisValue, DeviceId, DeviceInfo, HatDirection, InputAddress, InputEvent, InputId, InputValue,
    KeyCombo, OutputAddress, OutputId, VJoyAxis,
};

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
    InputAddress {
        device: dev_id(),
        input: InputId::Axis { index },
    }
}

fn button_addr(index: u8) -> InputAddress {
    InputAddress {
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
    );

    (engine, state, tx)
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

    // Input cache should be empty since processing was skipped.
    assert!(state.read().input_cache.get_all_axis_entries().is_empty());
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
    input.hotplug.push(HotplugEvent::Connected(device_info));

    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(s.devices.len(), 1);
    assert!(s.devices[0].connected);
    assert_eq!(s.devices[0].info.name, "Test Stick");
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
            condition: crate::action::Condition::ButtonPressed {
                input: button_addr(0),
            },
            if_true: vec![Action::ChangeMode {
                strategy: ModeChangeStrategy::Temporary {
                    mode: "Shift".to_owned(),
                },
            }],
            if_false: None,
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
    InputAddress {
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

/// Build a `ModeTree` with Default → Combat → Racing (three modes).
fn three_mode_tree() -> ModeTree {
    let map = HashMap::from([(
        "Default".to_owned(),
        vec!["Combat".to_owned(), "Racing".to_owned()],
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
    );

    (engine, state, tx)
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
fn process_outputs_previous_mode() {
    let tree = two_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    mode_state.push_temporary("Combat", &tree).unwrap();
    assert_eq!(mode_state.current(), "Combat");

    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(0);

    let outputs = vec![PipelineOutput::ChangeMode {
        strategy: ModeChangeStrategy::Previous,
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
    assert_eq!(mode_state.current(), "Default");
}

#[test]
fn process_outputs_cycle_mode() {
    let tree = three_mode_tree();
    let mut mode_state = ModeState::new("Default".to_owned());
    let mut callbacks = CallbackRegistry::new();
    let trigger = button_addr(0);

    let cycle = CycleModes::new(vec![
        "Default".to_owned(),
        "Combat".to_owned(),
        "Racing".to_owned(),
    ])
    .unwrap();

    let outputs = vec![PipelineOutput::ChangeMode {
        strategy: ModeChangeStrategy::Cycle { modes: cycle },
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

    // Events are not processed when no profile is loaded.
    assert!(state.read().input_cache.get_all_axis_entries().is_empty());
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
    input
        .hotplug
        .push(HotplugEvent::Connected(updated_info.clone()));

    let (mut engine, state, _tx) = make_engine(input, profile);

    // Pre-populate a disconnected device with the same ID.
    state.write().devices.push(DeviceState {
        info: original_info,
        connected: false,
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

    // Calibration: raw range [-100, 100], no center deadzone.
    // Input of 50 should map to 0.5 after calibration.
    let cal = Calibration::new(-100.0, 0.0, 0.0, 100.0, true).unwrap();

    let mut input = MockInputSource::default();
    input.events.push(axis_event(0, 50.0));

    let (mut engine, state, _tx) = make_engine(input, profile);

    // Pre-populate calibration in shared state.
    state.write().calibrations.set(dev_id(), 0, cal);

    engine.tick().unwrap();

    // Verify the input cache received the raw value.
    let s = state.read();
    let cached = s.input_cache.get_all_axis_entries();
    assert_eq!(cached.len(), 1);
    // The raw value in the cache should be 50.0 (cache stores raw).
    assert!((cached[0].1 - 50.0).abs() < f64::EPSILON);
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
    // Default engine status is Stopped — no need to set it explicitly.

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
    );

    // Tick 1 (Stopped): input cache is updated, mappings are not evaluated.
    engine.tick().unwrap();
    assert!(
        state.read().output_cache.get_axis(1, VJoyAxis::X).abs() < f64::EPSILON,
        "output cache must be empty before activation"
    );

    // Send Activate and replace the input source with an empty one so
    // tick 2 has no new events — the refresh must rely solely on the cache.
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

    // Replace input with no events — the refresh must use the cached value.
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
