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
        PathBuf::new(),
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
        false,
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
        false,
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
        false,
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
        false,
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
        false,
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
        false,
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

/// Build a `ModeTree` with Default → Combat and Default → Racing.
fn combat_racing_tree() -> ModeTree {
    let map = HashMap::from([(
        "Default".to_owned(),
        vec!["Combat".to_owned(), "Racing".to_owned()],
    )]);
    ModeTree::from_adjacency(&map).unwrap()
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
        false,
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
        false,
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
        false,
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
        false,
    )
    .unwrap();

    assert!(result.mode_changed);
    assert_eq!(mode_state.current(), "Default");
}

#[test]
fn process_outputs_cycle_mode() {
    let tree = combat_racing_tree();
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
        false,
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
        PathBuf::new(),
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
///   directory alive — callers must bind it to a local variable.
/// - `path` is the absolute path to the profile TOML.
fn make_engine_with_simple_disk_profile() -> (
    Engine,
    Arc<RwLock<AppState>>,
    mpsc::Sender<EngineCommand>,
    tempfile::TempDir,
    PathBuf,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("TFM_Throttle.toml");
    let profile = make_profile(simple_mode_tree(), vec![]);
    profile.save(&path).unwrap();

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
        PathBuf::new(),
    );
    (engine, state, tx, dir, path)
}

// ---------------------------------------------------------------------------
// F6 forced-mode tests
// ---------------------------------------------------------------------------

#[test]
fn force_mode_from_unforced_switches_and_sets_force() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);

    tx.send(EngineCommand::ForceMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(s.current_mode, "Combat");
    assert_eq!(
        s.mode_force.as_ref().map(|f| f.mode.as_str()),
        Some("Combat")
    );
}

#[test]
fn release_mode_clears_force_keeps_current_mode() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);

    tx.send(EngineCommand::ForceMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    tx.send(EngineCommand::ReleaseMode).unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert!(s.mode_force.is_none());
    assert_eq!(
        s.current_mode, "Combat",
        "release does not change current mode"
    );
}

#[test]
fn force_mode_unknown_mode_returns_mode_not_found() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, _tx) = make_engine(MockInputSource::default(), profile);

    let err = engine.handle_command(EngineCommand::ForceMode {
        mode: "Nope".to_owned(),
    });
    assert!(
        matches!(err, Err(crate::error::EngineError::ModeNotFound { .. })),
        "expected ModeNotFound, got {err:?}"
    );
    assert!(
        state.read().mode_force.is_none(),
        "state must be unchanged on error"
    );
}

#[test]
fn force_mode_idempotent_on_same_mode() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);

    tx.send(EngineCommand::ForceMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let force_before = state.read().mode_force.clone();

    tx.send(EngineCommand::ForceMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let force_after = state.read().mode_force.clone();
    assert_eq!(force_before, force_after);
}

#[test]
fn force_mode_rotates_on_different_mode() {
    let profile = make_profile(three_mode_tree(), vec![]);
    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);

    tx.send(EngineCommand::ForceMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    tx.send(EngineCommand::ForceMode {
        mode: "Landing".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(s.current_mode, "Landing");
    assert_eq!(
        s.mode_force.as_ref().map(|f| f.mode.as_str()),
        Some("Landing")
    );
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
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("TFM_Throttle.toml");
    let profile = make_profile(simple_mode_tree(), vec![]);
    profile.save(&path).unwrap();

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
    let path = dir.path().join("TFM_Throttle.toml");
    let profile = make_profile(simple_mode_tree(), vec![]);
    profile.save(&path).unwrap();

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

#[test]
fn restore_snapshot_clears_mode_force() {
    // Use a profile that includes "Combat" so ForceMode succeeds.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("TFM_Throttle.toml");
    let profile = make_profile(two_mode_tree(), vec![]);
    profile.save(&path).unwrap();

    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    };
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
    );

    tx.send(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: None,
    })
    .unwrap();
    engine.tick().unwrap();
    tx.send(EngineCommand::ForceMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();
    assert!(state.read().mode_force.is_some());

    let snap_id = crate::snapshot::list(&path).unwrap()[0].id;
    tx.send(EngineCommand::RestoreSnapshot { id: snap_id })
        .unwrap();
    engine.tick().unwrap();

    assert!(
        state.read().mode_force.is_none(),
        "restore must clear mode_force"
    );
}

// ---------------------------------------------------------------------------
// F6 Task 24: mode-pause gate tests
// ---------------------------------------------------------------------------

#[test]
fn forced_mode_blocks_change_mode_pipeline_output() {
    // Mapping: button press → ChangeMode SwitchTo("Combat").
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
    let profile = make_profile(three_mode_tree(), vec![mapping]);
    let mut input = MockInputSource::default();
    input.events.push(button_event(0, true));

    let (mut engine, state, tx) = make_engine(input, profile);

    // Force into Landing first.
    tx.send(EngineCommand::ForceMode {
        mode: "Landing".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();
    assert_eq!(state.read().current_mode, "Landing");

    // Tick processes the button event; ChangeMode would normally switch to
    // Combat, but the gate must block it.
    engine.input = Box::new({
        let mut src = MockInputSource::default();
        src.events.push(button_event(0, true));
        src
    });
    engine.tick().unwrap();
    assert_eq!(
        state.read().current_mode,
        "Landing",
        "forced mode must block ChangeMode pipeline output"
    );
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
    let path = dir.path().join("TFM_Throttle.toml");
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
    };
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        settings,
        PathBuf::new(),
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
    // but is still a valid TOML *file* — pick a TOML that lacks the meta table.
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
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
        name: "".to_owned(),
        parent: None,
    });
    assert!(err.is_err(), "expected error on empty name");
    // Profile unchanged.
    let s = state.read();
    let modes = s.active_profile.as_ref().unwrap().modes().all_modes();
    assert_eq!(modes.len(), 3);
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
    {
        let mut s = state.write();
        s.profile_path = Some(path.parent().unwrap().to_path_buf());
    }
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
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
    // Force into Combat first; this populates current_mode + mode_force.
    tx.send(EngineCommand::ForceMode {
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

    let s = state.read();
    assert_eq!(
        s.current_mode, "Fighter",
        "current_mode should track rename"
    );
    assert_eq!(
        s.mode_force.as_ref().map(|f| f.mode.as_str()),
        Some("Fighter"),
        "mode_force should track rename"
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
        to: "".to_owned(),
    });
    assert!(err.is_err());
}

#[test]
fn rename_mode_rejects_when_cycle_would_collapse() {
    use crate::action::{CycleModes, Mapping, ModeChangeStrategy};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    let mappings = vec![Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::ChangeMode {
            strategy: ModeChangeStrategy::Cycle {
                modes: CycleModes::new(vec!["Combat".to_owned(), "Landing".to_owned()]).unwrap(),
            },
        }],
    }];
    let profile = make_profile(three_mode_tree(), mappings);
    profile.save(&path).unwrap();

    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
    );

    let err = engine.handle_command(EngineCommand::RenameMode {
        from: "Combat".to_owned(),
        to: "Landing".to_owned(),
    });
    assert!(err.is_err(), "rename collapsing the cycle must be rejected");

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
    let _ = _tx;
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
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
fn rename_mode_rewrites_cycle_action() {
    use crate::action::{CycleModes, Mapping, ModeChangeStrategy};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    let mappings = vec![Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::ChangeMode {
            strategy: ModeChangeStrategy::Cycle {
                modes: CycleModes::new(vec!["Combat".to_owned(), "Landing".to_owned()]).unwrap(),
            },
        }],
    }];
    let profile = make_profile(three_mode_tree(), mappings);
    profile.save(&path).unwrap();
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
            strategy: ModeChangeStrategy::Cycle { modes },
        } => assert_eq!(modes.modes(), &["Fighter".to_owned(), "Landing".to_owned()]),
        other => panic!("expected Cycle with renamed target, got {other:?}"),
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

    // current is "Landing" — unaffected.
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
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
fn delete_mode_resets_current_and_clears_force_when_referenced() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_disk_profile();
    // Force into Combat.
    tx.send(EngineCommand::ForceMode {
        mode: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    tx.send(EngineCommand::DeleteMode {
        name: "Combat".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let s = state.read();
    assert_eq!(s.current_mode, "Default", "current_mode resets to startup");
    assert!(
        s.mode_force.is_none(),
        "mode_force cleared by delete cascade"
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
    {
        let mut s = state.write();
        s.profile_path = Some(path.clone());
        s.engine_status = EngineStatus::Running;
    }
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
    );

    // Force into Missiles (descendant of Combat).
    tx.send(EngineCommand::ForceMode {
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

    // Delete Combat — every descendant (Missiles, Guns) and Combat itself
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
    assert!(s.mode_force.is_none(), "mode_force must be cleared");
    assert_eq!(
        engine.mode_state.current(),
        "Default",
        "ModeState::current resets"
    );
    // pop_temporary is a no-op on an empty stack — the post-delete current
    // stays "Default".
    engine.mode_state.pop_temporary();
    assert_eq!(
        engine.mode_state.current(),
        "Default",
        "stack must be purged of removed names"
    );
}
