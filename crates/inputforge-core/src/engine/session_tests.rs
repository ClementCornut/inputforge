use super::*;
use crate::{
    action::Action,
    device::{HotplugEvent, InputUpdate},
    error::{EngineError, Result},
    mode::Modes,
    output::{
        MockKeyboardSink, MockMouseSink, OutputFailure, OutputKind, OutputPhase,
        traits::ControllerCapabilities,
    },
    profile::{
        Profile,
        controllers::{ControllerConfig, DeviceBinding},
    },
    state::EngineStatus,
    types::*,
};
use parking_lot::Mutex;
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Failure {
    Acquire,
    Flush,
    Release,
    KeyboardStart,
    MouseStart,
    KeyboardStop,
    MouseStop,
    OutputStop,
}

#[derive(Default)]
struct Script {
    calls: Vec<String>,
    buttons: Vec<(u8, bool)>,
    axes: Vec<(u8, VJoyAxis, f64)>,
    start_failures_remaining: usize,
    acquired: Vec<Vec<DeviceId>>,
    polls: VecDeque<Vec<InputUpdate>>,
    rejected_axis_device: Option<u8>,
    failures: BTreeSet<Failure>,
    keyboard_start_output_failure: Option<OutputFailure>,
    keyboard_stop_output_failure: Option<OutputFailure>,
    mouse_stop_output_failure: Option<OutputFailure>,
}
struct Input(Arc<Mutex<Script>>);
impl InputSource for Input {
    fn supports_exclusive(&self) -> bool {
        true
    }
    fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        vec![]
    }
    fn is_device_connected(&self, _: &DeviceId) -> bool {
        true
    }
    fn hotplug_events(&mut self) -> Vec<HotplugEvent> {
        vec![]
    }
    fn acquire(&mut self, ids: &[DeviceId]) -> Result<()> {
        let mut s = self.0.lock();
        s.acquired.push(ids.to_vec());
        s.calls.push("acquire".into());
        if s.failures.contains(&Failure::Acquire) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn release(&mut self) -> Result<()> {
        let mut s = self.0.lock();
        s.calls.push("release".into());
        if s.failures.contains(&Failure::Release) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn poll(&mut self, out: &mut Vec<InputUpdate>) -> Result<()> {
        let mut s = self.0.lock();
        s.calls.push("poll".into());
        out.extend(s.polls.pop_front().unwrap_or_default());
        Ok(())
    }
}
struct Output {
    script: Arc<Mutex<Script>>,
    fixed_layout: Option<Vec<VirtualDeviceConfig>>,
}
impl OutputSink for Output {
    fn capabilities(&self) -> ControllerCapabilities {
        ControllerCapabilities {
            configurable: self.fixed_layout.is_none(),
            ..ControllerCapabilities::default()
        }
    }
    fn list_devices(&self) -> Vec<VirtualDeviceConfig> {
        self.fixed_layout.clone().unwrap_or_default()
    }
    fn start(&mut self, _: &[VirtualDeviceConfig]) -> Result<()> {
        let mut script = self.script.lock();
        script.calls.push("start".into());
        if script.start_failures_remaining > 0 {
            script.start_failures_remaining -= 1;
            Err(error())
        } else {
            Ok(())
        }
    }
    fn neutralize(&mut self) -> Result<()> {
        self.script.lock().calls.push("neutral".into());
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        let mut script = self.script.lock();
        script.calls.push("stop".into());
        if script.failures.contains(&Failure::OutputStop) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()> {
        let mut script = self.script.lock();
        script.calls.push(format!("axis:{value}"));
        script.axes.push((device, axis, value));
        if script.rejected_axis_device == Some(device) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn set_button(&mut self, _: u8, button: u8, pressed: bool) -> Result<()> {
        self.script.lock().calls.push(format!("button:{pressed}"));
        self.script.lock().buttons.push((button, pressed));
        Ok(())
    }
    fn set_hat(&mut self, _: u8, _: u8, direction: HatDirection) -> Result<()> {
        self.script.lock().calls.push(format!("hat:{direction:?}"));
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        let mut s = self.script.lock();
        s.calls.push("flush".into());
        if s.failures.contains(&Failure::Flush) {
            Err(error())
        } else {
            Ok(())
        }
    }
}

struct Keyboard(Arc<Mutex<Script>>);
impl KeyboardSink for Keyboard {
    fn start(&mut self) -> Result<()> {
        let mut script = self.0.lock();
        script.calls.push("keyboard:start".into());
        if let Some(failure) = &script.keyboard_start_output_failure {
            return Err(EngineError::InjectionFailed {
                failure: failure.clone(),
            });
        }
        if script.failures.contains(&Failure::KeyboardStart) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn stop(&mut self) -> Result<()> {
        let mut script = self.0.lock();
        script.calls.push("keyboard:stop".into());
        if let Some(failure) = &script.keyboard_stop_output_failure {
            return Err(EngineError::InjectionFailed {
                failure: failure.clone(),
            });
        }
        if script.failures.contains(&Failure::KeyboardStop) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn key_down(&mut self, _: &KeyCombo) -> Result<()> {
        self.0.lock().calls.push("keyboard:down".into());
        Ok(())
    }
    fn key_up(&mut self, _: &KeyCombo) -> Result<()> {
        self.0.lock().calls.push("keyboard:up".into());
        Ok(())
    }
}

struct Mouse(Arc<Mutex<Script>>);
impl MouseSink for Mouse {
    fn start(&mut self) -> Result<()> {
        let mut script = self.0.lock();
        script.calls.push("mouse:start".into());
        if script.failures.contains(&Failure::MouseStart) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn stop(&mut self) -> Result<()> {
        let mut script = self.0.lock();
        script.calls.push("mouse:stop".into());
        if let Some(failure) = &script.mouse_stop_output_failure {
            return Err(EngineError::InjectionFailed {
                failure: failure.clone(),
            });
        }
        if script.failures.contains(&Failure::MouseStop) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn button_down(&mut self, _: crate::action::MouseTarget) -> Result<()> {
        self.0.lock().calls.push("mouse:down".into());
        Ok(())
    }
    fn button_up(&mut self, _: crate::action::MouseTarget) -> Result<()> {
        self.0.lock().calls.push("mouse:up".into());
        Ok(())
    }
    fn wheel(&mut self, _: crate::action::MouseTarget) -> Result<()> {
        self.0.lock().calls.push("mouse:wheel".into());
        Ok(())
    }
}
fn error() -> EngineError {
    EngineError::OutputFailed {
        reason: "injected".into(),
    }
}
fn button(pressed: bool) -> InputEvent {
    InputEvent {
        source: InputAddress::Bound {
            device: DeviceId("evdev:v1:test".into()),
            input: InputId::Button { index: 0 },
        },
        value: InputValue::Button { pressed },
        timestamp: Instant::now(),
    }
}
fn snapshot(pressed: bool) -> InputUpdate {
    InputUpdate::Snapshot {
        device: DeviceId("evdev:v1:test".into()),
        values: vec![button(pressed)],
        recovered: false,
    }
}
fn harness() -> (
    Engine,
    mpsc::Sender<EngineCommand>,
    Arc<Mutex<Script>>,
    tempfile::TempDir,
) {
    harness_with_output(None)
}

fn fixed_harness(
    native_layout: Vec<VirtualDeviceConfig>,
) -> (
    Engine,
    mpsc::Sender<EngineCommand>,
    Arc<Mutex<Script>>,
    tempfile::TempDir,
) {
    harness_with_output(Some(native_layout))
}

fn harness_with_output(
    fixed_layout: Option<Vec<VirtualDeviceConfig>>,
) -> (
    Engine,
    mpsc::Sender<EngineCommand>,
    Arc<Mutex<Script>>,
    tempfile::TempDir,
) {
    let script = Arc::new(Mutex::new(Script::default()));
    harness_with_sinks(
        fixed_layout,
        Arc::clone(&script),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockMouseSink::new()),
    )
}

fn lifecycle_harness() -> (
    Engine,
    mpsc::Sender<EngineCommand>,
    Arc<Mutex<Script>>,
    tempfile::TempDir,
) {
    let script = Arc::new(Mutex::new(Script::default()));
    harness_with_sinks(
        None,
        Arc::clone(&script),
        Box::new(Keyboard(Arc::clone(&script))),
        Box::new(Mouse(Arc::clone(&script))),
    )
}

fn harness_with_sinks(
    fixed_layout: Option<Vec<VirtualDeviceConfig>>,
    script: Arc<Mutex<Script>>,
    keyboard: Box<dyn KeyboardSink>,
    mouse: Box<dyn MouseSink>,
) -> (
    Engine,
    mpsc::Sender<EngineCommand>,
    Arc<Mutex<Script>>,
    tempfile::TempDir,
) {
    let mut profile = Profile::new(
        "test".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    profile
        .set_controllers(ControllerConfig {
            legacy_axis_settings: false,
            version: 1,
            selected: vec![DeviceId("evdev:v1:test".into())],
            bindings: vec![DeviceBinding {
                observed_layout: None,
                unavailable: vec![],
                device: DeviceId("evdev:v1:test".into()),
                axes: vec![],
                buttons: vec![704],
                hats: vec![],
            }],
            virtual_devices: vec![VirtualDeviceConfig {
                device_id: 1,
                axes: vec![],
                button_count: 2,
                hat_count: 0,
            }],
        })
        .unwrap();
    profile.set_mapping(
        &button(false).source,
        "Default",
        None,
        vec![Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Button { id: 1 },
            },
        }],
    );
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let (tx, rx) = mpsc::channel();
    let temp = tempfile::tempdir().unwrap();
    let engine = Engine::new(
        Box::new(Input(Arc::clone(&script))),
        Box::new(Output {
            script: Arc::clone(&script),
            fixed_layout,
        }),
        keyboard,
        mouse,
        state,
        rx,
        AppSettings::default(),
        temp.path().join("settings.toml"),
        Box::new(inputforge_autostart::mock::MockAutostart::default()),
    );
    (engine, tx, script, temp)
}

fn set_session_devices(
    engine: &Engine,
    selected: Vec<DeviceId>,
    virtual_devices: Vec<VirtualDeviceConfig>,
) {
    let mut state = engine.state.write();
    let profile = state.active_profile.as_mut().unwrap();
    let mut controllers = profile.controllers().unwrap().clone();
    controllers.selected = selected;
    controllers.virtual_devices = virtual_devices;
    profile.set_controllers(controllers).unwrap();
}

fn injection_failure(
    output: OutputKind,
    phase: OutputPhase,
    details: &str,
    cleanup: &[&str],
) -> OutputFailure {
    OutputFailure {
        output,
        phase,
        category: std::io::ErrorKind::BrokenPipe,
        details: details.into(),
        cleanup: cleanup.iter().map(|detail| (*detail).into()).collect(),
    }
}

#[test]
fn output_failure_primary_and_secondary_cleanup_are_retained() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    script.lock().keyboard_start_output_failure = Some(injection_failure(
        OutputKind::Keyboard,
        OutputPhase::Initialization,
        "primary keyboard failure",
        &[],
    ));
    script.lock().keyboard_stop_output_failure = Some(injection_failure(
        OutputKind::Keyboard,
        OutputPhase::Release,
        "secondary keyboard cleanup",
        &["native close cleanup"],
    ));

    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    let state = engine.state.read();
    assert_eq!(state.session.output_failures.len(), 1);
    let failure = &state.session.output_failures[0];
    assert_eq!(failure.details, "primary keyboard failure");
    assert!(
        failure
            .cleanup
            .iter()
            .any(|detail| detail.contains("secondary keyboard cleanup"))
    );
    assert!(
        failure
            .cleanup
            .iter()
            .any(|detail| detail.contains("native close cleanup"))
    );
}

#[test]
fn output_failure_command_publication_does_not_overwrite_fault_details() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    script.lock().keyboard_start_output_failure = Some(injection_failure(
        OutputKind::Keyboard,
        OutputPhase::Initialization,
        "primary keyboard failure",
        &[],
    ));
    script.lock().mouse_stop_output_failure = Some(injection_failure(
        OutputKind::Mouse,
        OutputPhase::Release,
        "mouse cleanup failure",
        &[],
    ));

    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    let message = engine.state.read().session.error.clone().unwrap();
    assert!(message.contains("primary keyboard failure"));
    assert!(message.contains("mouse cleanup failure"));
}

#[test]
fn output_failure_persists_across_stop_refresh_and_profile_edits() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    script.lock().keyboard_start_output_failure = Some(injection_failure(
        OutputKind::Keyboard,
        OutputPhase::Initialization,
        "persistent failure",
        &[],
    ));
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    script.lock().keyboard_start_output_failure = None;
    script.lock().calls.clear();

    tx.send(EngineCommand::Deactivate).unwrap();
    tx.send(EngineCommand::RefreshInput).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.state.read().session.output_failures.len(), 1);
    assert!(engine.state.read().session.error.is_some());

    engine
        .handle_session_command(&EngineCommand::LoadProfile("replacement.toml".into()))
        .unwrap();
    assert_eq!(engine.state.read().session.output_failures.len(), 1);
    assert!(engine.state.read().session.error.is_some());
}

#[test]
fn output_failure_successful_retry_is_the_only_clearing_path() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    script.lock().keyboard_start_output_failure = Some(injection_failure(
        OutputKind::Keyboard,
        OutputPhase::Initialization,
        "retryable failure",
        &[],
    ));
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.state.read().session.output_failures.len(), 1);

    tx.send(EngineCommand::Retry).unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.state.read().session.output_failures.len(), 1);

    script.lock().keyboard_start_output_failure = None;
    tx.send(EngineCommand::Retry).unwrap();
    engine.tick().unwrap();
    let state = engine.state.read();
    assert!(state.session.output_failures.is_empty());
    assert!(state.session.error.is_none());
    assert_eq!(state.engine_status, EngineStatus::Running);
}

#[test]
fn injection_start_order_with_and_without_virtual_controllers() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    assert_eq!(
        &script.lock().calls[..5],
        [
            "keyboard:start",
            "mouse:start",
            "start",
            "neutral",
            "acquire"
        ]
    );

    let (mut engine, tx, script, _temp) = lifecycle_harness();
    set_session_devices(&engine, vec![DeviceId("evdev:v1:test".into())], vec![]);
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    assert_eq!(
        &script.lock().calls[..3],
        ["keyboard:start", "mouse:start", "acquire"]
    );
    assert!(!script.lock().calls.iter().any(|call| call == "start"));
}

#[test]
fn mouse_start_failure_rolls_back_keyboard_before_any_controller_or_capture() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    script.lock().failures.insert(Failure::MouseStart);
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();

    let calls = script.lock().calls.clone();
    assert_eq!(
        &calls[..6],
        [
            "keyboard:start",
            "mouse:start",
            "release",
            "keyboard:stop",
            "mouse:stop",
            "stop",
        ]
    );
    assert!(
        !calls
            .iter()
            .any(|call| call == "start" || call == "acquire")
    );
    assert_eq!(engine.read_status(), EngineStatus::Faulted);
}

#[test]
fn controller_start_and_input_capture_failures_roll_back_both_injection_sinks() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    script.lock().start_failures_remaining = 1;
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    let calls = script.lock().calls.clone();
    assert!(
        calls
            .windows(3)
            .any(|window| { window == ["keyboard:stop", "mouse:stop", "stop"] })
    );
    assert!(!calls.iter().any(|call| call == "acquire"));

    let (mut engine, tx, script, _temp) = lifecycle_harness();
    script.lock().failures.insert(Failure::Acquire);
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    let calls = script.lock().calls.clone();
    assert!(
        calls
            .windows(4)
            .any(|window| { window == ["release", "keyboard:stop", "mouse:stop", "stop"] })
    );
}

#[test]
fn stop_closes_injection_sinks_and_retains_only_clean_neutral_controllers() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    script.lock().calls.clear();

    tx.send(EngineCommand::Deactivate).unwrap();
    engine.tick().unwrap();
    assert_eq!(
        &script.lock().calls[..4],
        ["release", "keyboard:stop", "mouse:stop", "neutral"]
    );
    assert!(!script.lock().calls.iter().any(|call| call == "stop"));
    assert!(engine.state.read().session.output_active);
}

#[test]
fn cleanup_failure_calls_both_sink_stops_and_destroys_controllers() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    script.lock().calls.clear();
    script.lock().failures.insert(Failure::KeyboardStop);
    script.lock().failures.insert(Failure::MouseStop);

    tx.send(EngineCommand::Deactivate).unwrap();
    engine.tick().unwrap();
    let calls = script.lock().calls.clone();
    assert!(calls.iter().any(|call| call == "keyboard:stop"));
    assert!(calls.iter().any(|call| call == "mouse:stop"));
    assert!(calls.iter().any(|call| call == "stop"));
    assert!(!calls.iter().any(|call| call == "neutral"));
    assert!(!engine.state.read().session.output_active);
}

#[test]
fn repeated_start_stop_retry_restarts_injection_but_reuses_controllers() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    script.lock().calls.clear();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    assert!(
        !script
            .lock()
            .calls
            .iter()
            .any(|call| call.ends_with(":start"))
    );

    tx.send(EngineCommand::Deactivate).unwrap();
    engine.tick().unwrap();
    script.lock().calls.clear();
    tx.send(EngineCommand::Retry).unwrap();
    engine.tick().unwrap();
    let calls = script.lock().calls.clone();
    assert_eq!(&calls[..3], ["keyboard:start", "mouse:start", "acquire"]);
    assert!(!calls.iter().any(|call| call == "start"));
}

#[test]
fn mapping_and_mode_edits_do_not_stop_healthy_injection_sinks() {
    let (mut engine, tx, script, _temp) = lifecycle_harness();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    script.lock().calls.clear();

    engine.release_all_held_outputs().unwrap();
    engine
        .handle_command(EngineCommand::SwitchMode {
            mode: "Default".into(),
        })
        .unwrap();
    assert!(
        !script
            .lock()
            .calls
            .iter()
            .any(|call| call.ends_with(":stop"))
    );
}

#[test]
fn profile_replacement_and_active_deletion_close_injection_sinks() {
    for command in [
        EngineCommand::LoadProfile("replacement.toml".into()),
        EngineCommand::DeleteProfile {
            name: "test".into(),
        },
    ] {
        let (mut engine, tx, script, _temp) = lifecycle_harness();
        tx.send(EngineCommand::Activate).unwrap();
        engine.tick().unwrap();
        script.lock().calls.clear();

        assert!(!engine.handle_session_command(&command).unwrap());
        let calls = script.lock().calls.clone();
        assert!(calls.iter().any(|call| call == "keyboard:stop"));
        assert!(calls.iter().any(|call| call == "mouse:stop"));
    }
}

#[test]
fn shutdown_disconnect_and_drop_close_injection_sinks() {
    let verify = |calls: &[String]| {
        assert!(calls.iter().any(|call| call == "keyboard:stop"));
        assert!(calls.iter().any(|call| call == "mouse:stop"));
        assert!(calls.iter().any(|call| call == "stop"));
    };

    let (mut engine, tx, script, _temp) = lifecycle_harness();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    script.lock().calls.clear();
    tx.send(EngineCommand::Shutdown).unwrap();
    engine.tick().unwrap();
    verify(&script.lock().calls);

    let (mut engine, tx, script, _temp) = lifecycle_harness();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    script.lock().calls.clear();
    drop(tx);
    engine.tick().unwrap();
    verify(&script.lock().calls);

    let (mut engine, tx, script, _temp) = lifecycle_harness();
    tx.send(EngineCommand::Activate).unwrap();
    engine.tick().unwrap();
    script.lock().calls.clear();
    drop(engine);
    verify(&script.lock().calls);
}

#[test]
fn passive_snapshot_blocks_held_controls_when_routing_starts_and_stop_keeps_monitoring() {
    let (mut e, tx, s, _temp) = harness();
    e.tick().unwrap();
    assert_eq!(e.state.read().engine_status, EngineStatus::Stopped);
    assert_eq!(s.lock().calls, ["poll"]);
    s.lock().polls.push_back(vec![snapshot(true)]);
    e.tick().unwrap();
    assert_eq!(e.state.read().engine_status, EngineStatus::Stopped);
    assert_eq!(
        e.state.read().session.monitored,
        [DeviceId("evdev:v1:test".into())]
    );
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![]);
    e.tick().unwrap();
    assert_eq!(s.lock().acquired, [vec![DeviceId("evdev:v1:test".into())]]);
    assert!(!s.lock().calls.iter().any(|c| c == "button:true"));
    s.lock().polls.push_back(vec![InputUpdate::Frame(vec![
        button(false),
        button(true),
        button(false),
    ])]);
    e.tick().unwrap();
    assert_eq!(
        s.lock()
            .calls
            .iter()
            .filter(|c| c.starts_with("button:"))
            .cloned()
            .collect::<Vec<_>>(),
        ["button:true", "button:false"]
    );
    s.lock().calls.clear();
    tx.send(EngineCommand::Deactivate).unwrap();
    e.tick().unwrap();
    assert_eq!(&s.lock().calls[..2], ["release", "neutral"]);
    assert!(!s.lock().calls.iter().any(|c| c == "stop"));
    assert!(e.state.read().session.captured.is_empty());
}

#[test]
fn failure_and_shutdown_release_before_output_cleanup_and_do_not_poll_again() {
    let (mut e, tx, s, _temp) = harness();
    s.lock().failures.insert(Failure::Acquire);
    tx.send(EngineCommand::Activate).unwrap();
    e.tick().unwrap();
    assert_eq!(e.state.read().engine_status, EngineStatus::Faulted);
    assert!(s.lock().calls.windows(2).any(|w| w == ["release", "stop"]));
    s.lock().calls.clear();
    tx.send(EngineCommand::Shutdown).unwrap();
    tx.send(EngineCommand::Activate).unwrap();
    e.tick().unwrap();
    assert!(!s.lock().calls.iter().any(|c| c == "poll" || c == "acquire"));
    assert!(e.shutdown);
}

#[test]
fn reset_recovery_in_one_poll_changes_generation_without_synthesizing_press() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    let generation = e.state.read().session.generation;
    s.lock().calls.clear();
    s.lock().polls.push_back(vec![
        InputUpdate::Reset {
            device: DeviceId("evdev:v1:test".into()),
        },
        snapshot(true),
    ]);
    e.tick().unwrap();
    assert!(e.state.read().session.generation > generation);
    assert!(!s.lock().calls.iter().any(|c| c.starts_with("button:")));
    assert_eq!(e.state.read().engine_status, EngineStatus::Running);
}

#[test]
fn release_failure_still_stops_outputs_and_drop_releases_first() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    s.lock().calls.clear();
    s.lock().failures.insert(Failure::Release);
    tx.send(EngineCommand::Deactivate).unwrap();
    e.tick().unwrap();
    assert_eq!(e.state.read().engine_status, EngineStatus::Faulted);
    assert_eq!(&s.lock().calls[..2], ["release", "stop"]);
    s.lock().calls.clear();
    tx.send(EngineCommand::Retry).unwrap();
    e.tick().unwrap();
    assert_eq!(&s.lock().calls[..2], ["release", "stop"]);
    assert!(
        !s.lock()
            .calls
            .iter()
            .any(|call| call == "start" || call == "acquire")
    );
    assert_eq!(e.state.read().engine_status, EngineStatus::Faulted);
    s.lock().calls.clear();
    drop(e);
    assert_eq!(&s.lock().calls[..2], ["release", "stop"]);
}

#[test]
fn secondary_changes_later_in_batch_cannot_retroactively_enable_press() {
    let (mut e, tx, s, _temp) = harness();
    let secondary = InputAddress::Bound {
        device: DeviceId("evdev:v1:test".into()),
        input: InputId::Button { index: 1 },
    };
    let mut state = e.state.write();
    let p = state.active_profile.as_mut().unwrap();
    let mut linux = p.controllers().unwrap().clone();
    linux.bindings[0].buttons.push(705);
    p.set_controllers(linux).unwrap();
    p.set_mapping(
        &button(false).source,
        "Default",
        None,
        vec![Action::Conditional {
            condition: crate::action::Condition::ButtonPressed {
                input: secondary.clone(),
            },
            if_true: vec![Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Button { id: 1 },
                },
            }],
            if_false: vec![],
        }],
    );
    drop(state);
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    s.lock().calls.clear();
    let mut condition_event = button(true);
    condition_event.source = secondary;
    s.lock().polls.push_back(vec![InputUpdate::Frame(vec![
        button(true),
        button(false),
        condition_event,
    ])]);
    e.tick().unwrap();
    assert!(!s.lock().calls.iter().any(|c| c == "button:true"));
}

#[test]
fn stop_discards_pending_controller_state_before_any_individual_release_write() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![button(true)])]);
    e.tick().unwrap();
    s.lock().calls.clear();
    tx.send(EngineCommand::Deactivate).unwrap();
    e.tick().unwrap();
    assert_eq!(&s.lock().calls[..2], ["release", "neutral"]);
    assert!(!s.lock().calls.iter().any(|c| c.starts_with("button:")));
}

#[test]
fn stop_restores_the_permanent_mode_immediately() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    let modes = Modes::new(vec!["Default".into(), "Temporary".into()]).unwrap();
    e.mode_state.push_temporary("Temporary", &modes).unwrap();
    e.state.write().current_mode = "Temporary".into();
    tx.send(EngineCommand::Deactivate).unwrap();
    e.tick().unwrap();
    assert_eq!(e.mode_state.current(), "Default");
    assert_eq!(e.state.read().current_mode, "Default");
}

#[test]
fn restart_retains_outputs_and_direct_flush_failure_releases_the_whole_session() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    tx.send(EngineCommand::Deactivate).unwrap();
    e.tick().unwrap();
    s.lock().calls.clear();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(true)]);
    e.tick().unwrap();
    assert!(
        !s.lock()
            .calls
            .iter()
            .any(|c| c == "start" || c == "button:true")
    );
    assert_eq!(s.lock().calls[0], "acquire");
    s.lock().calls.clear();
    s.lock().failures.insert(Failure::Flush);
    s.lock().polls.push_back(vec![]);
    e.tick().unwrap_err();
    assert!(s.lock().calls.ends_with(&["release".into(), "stop".into()]));
    assert_eq!(e.state.read().engine_status, EngineStatus::Faulted);
    assert!(e.state.read().session.captured.is_empty());
    s.lock().failures.remove(&Failure::Flush);
    s.lock().calls.clear();
    tx.send(EngineCommand::Retry).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    assert_eq!(
        &s.lock().calls[..5],
        ["release", "stop", "start", "neutral", "acquire"]
    );
    s.lock().calls.clear();
    drop(tx);
    e.tick().unwrap();
    assert_eq!(s.lock().calls, ["release", "stop"]);
    assert!(e.shutdown);
}

#[test]
fn post_snapshot_edges_in_the_same_poll_are_not_lost() {
    for recovering in [false, true] {
        let (mut e, tx, s, _temp) = harness();
        tx.send(EngineCommand::Activate).unwrap();
        if recovering {
            s.lock().polls.push_back(vec![snapshot(false)]);
            e.tick().unwrap();
            s.lock().calls.clear();
        }
        let mut updates = vec![];
        if recovering {
            updates.push(InputUpdate::Reset {
                device: DeviceId("evdev:v1:test".into()),
            });
        }
        updates.extend([
            snapshot(false),
            InputUpdate::Frame(vec![button(true), button(false)]),
        ]);
        s.lock().polls.push_back(updates);
        e.tick().unwrap();
        let calls: Vec<_> = s
            .lock()
            .calls
            .iter()
            .filter(|c| c.starts_with("button:"))
            .cloned()
            .collect();
        assert_eq!(calls, ["button:true", "button:false"]);
    }
}

#[test]
fn sampled_hat_requires_its_own_physical_change() {
    let (mut e, tx, s, _temp) = harness();
    let hat = InputAddress::Bound {
        device: DeviceId("evdev:v1:test".into()),
        input: InputId::Hat { index: 0 },
    };
    let mut state = e.state.write();
    let profile = state.active_profile.as_mut().unwrap();
    let mut config = profile.controllers().unwrap().clone();
    config.bindings[0].hats.push(0);
    config.virtual_devices[0].hat_count = 1;
    profile.set_controllers(config).unwrap();
    profile.set_mapping(
        &hat,
        "Default",
        None,
        vec![Action::Conditional {
            condition: crate::action::Condition::ButtonPressed {
                input: button(false).source,
            },
            if_true: vec![Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Hat { id: 1 },
                },
            }],
            if_false: vec![],
        }],
    );
    drop(state);
    let event = InputEvent {
        source: hat,
        value: InputValue::Hat {
            direction: HatDirection::N,
        },
        timestamp: Instant::now(),
    };
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![InputUpdate::Snapshot {
        device: DeviceId("evdev:v1:test".into()),
        values: vec![button(false), event.clone()],
        recovered: false,
    }]);
    e.tick().unwrap();
    s.lock().calls.clear();
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![button(true)])]);
    e.tick().unwrap();
    assert!(!s.lock().calls.iter().any(|c| c.starts_with("hat:")));
    let mut changed = event;
    changed.value = InputValue::Hat {
        direction: HatDirection::E,
    };
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![changed])]);
    e.tick().unwrap();
    assert!(s.lock().calls.iter().any(|c| c == "hat:E"));
}

#[test]
fn monitoring_without_profile_has_values_and_never_starts_outputs_or_grabs() {
    let (mut e, _tx, s, _temp) = harness();
    e.state.write().active_profile = None;
    s.lock().polls.push_back(vec![snapshot(true)]);
    e.tick().unwrap();
    assert_eq!(s.lock().calls, ["poll"]);
    let state = e.state.read();
    assert_eq!(state.session.monitored, [DeviceId("evdev:v1:test".into())]);
    assert_eq!(state.input_cache.clone_compact().len(), 1);
    assert!(state.session.captured.is_empty());
    assert!(!state.session.output_active);
}

#[test]
fn invalid_mapping_leaves_valid_mapping_running() {
    let (mut e, tx, s, _temp) = harness();
    let mut invalid_event = button(true);
    invalid_event.source = InputAddress::Bound {
        device: DeviceId("evdev:v1:test".into()),
        input: InputId::Button { index: 1 },
    };
    {
        let mut state = e.state.write();
        let profile = state.active_profile.as_mut().unwrap();
        let mut config = profile.controllers().unwrap().clone();
        config.bindings[0].buttons.push(705);
        profile.set_controllers(config).unwrap();
        profile.set_mapping(
            &invalid_event.source,
            "Default",
            None,
            vec![Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Button { id: 99 },
                },
            }],
        );
    };
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    s.lock().calls.clear();
    s.lock().polls.push_back(vec![InputUpdate::Frame(vec![
        button(true),
        invalid_event.clone(),
    ])]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true)]);
    assert!(
        !s.lock()
            .calls
            .iter()
            .any(|call| call == "release" || call == "stop")
    );
    let state = e.state.read();
    assert_eq!(state.engine_status, EngineStatus::Running);
    assert_eq!(state.session.mapping_issues.len(), 1);
    assert_eq!(state.session.mapping_issues[0].input, invalid_event.source);
}

#[test]
fn live_mapping_edit_releases_old_output_and_keeps_capture_and_monitoring() {
    let (mut e, tx, s, temp) = harness();
    e.state.write().profile_path = Some(temp.path().join("profile.toml"));
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![
        snapshot(false),
        InputUpdate::Frame(vec![button(true)]),
    ]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true)]);
    s.lock().calls.clear();
    tx.send(EngineCommand::SetMapping {
        input: button(false).source,
        mode: "Default".into(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Button { id: 2 },
            },
        }],
    })
    .unwrap();
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true), (1, false)]);
    assert!(
        !s.lock()
            .calls
            .iter()
            .any(|call| call == "release" || call == "stop" || call == "acquire")
    );
    assert_eq!(e.state.read().engine_status, EngineStatus::Running);
    assert_eq!(
        e.state.read().session.monitored,
        [DeviceId("evdev:v1:test".into())]
    );
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![button(false), button(true)])]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons.last(), Some(&(2, true)));
}

#[test]
fn output_failure_keeps_passive_values_updating_without_reacquiring() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    s.lock().failures.insert(Failure::Flush);
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![button(true)])]);
    e.tick().unwrap_err();
    s.lock().calls.clear();
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![button(false)])]);
    e.tick().unwrap();
    assert_eq!(s.lock().calls, ["poll"]);
    let state = e.state.read();
    assert_eq!(state.engine_status, EngineStatus::Faulted);
    assert!(state.session.captured.is_empty());
    assert_eq!(state.session.monitored, [DeviceId("evdev:v1:test".into())]);
    assert_eq!(
        state.input_cache.clone_compact()[0].value,
        InputValue::Button { pressed: false }
    );
}

#[test]
fn resetting_last_monitored_device_clears_readiness_and_held_output() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![
        snapshot(false),
        InputUpdate::Frame(vec![button(true)]),
    ]);
    e.tick().unwrap();
    s.lock().polls.push_back(vec![InputUpdate::Reset {
        device: DeviceId("evdev:v1:test".into()),
    }]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true), (1, false)]);
    let state = e.state.read();
    assert!(state.session.monitored.is_empty());
    assert!(!state.session.ready);
    assert!(state.input_cache.clone_compact().is_empty());
}

#[test]
fn reset_releases_temporary_mode_callback_without_replaying_snapshot_press() {
    let (mut e, tx, s, _temp) = harness();
    let mut profile = Profile::new(
        "test".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Temporary".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    profile
        .set_controllers(
            e.state
                .read()
                .active_profile
                .as_ref()
                .unwrap()
                .controllers()
                .unwrap()
                .clone(),
        )
        .unwrap();
    profile.set_mapping(
        &button(false).source,
        "Default",
        None,
        vec![Action::ChangeMode {
            strategy: crate::action::ModeChangeStrategy::Temporary {
                mode: "Temporary".into(),
            },
        }],
    );
    e.state.write().active_profile = Some(profile);
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![
        snapshot(false),
        InputUpdate::Frame(vec![button(true)]),
    ]);
    e.tick().unwrap();
    assert_eq!(e.mode_state.current(), "Temporary");
    s.lock().polls.push_back(vec![
        InputUpdate::Reset {
            device: DeviceId("evdev:v1:test".into()),
        },
        snapshot(true),
    ]);
    e.tick().unwrap();
    assert_eq!(e.mode_state.current(), "Default");
    assert_eq!(e.state.read().current_mode, "Default");
    assert!(e.callbacks.fire(&button(false).source).is_empty());
}

#[test]
fn invalid_axis_target_is_not_written_during_mapping_invalidation() {
    let (mut e, tx, s, _temp) = harness();
    {
        let mut state = e.state.write();
        let profile = state.active_profile.as_mut().unwrap();
        let mut config = profile.controllers().unwrap().clone();
        config.bindings[0]
            .axes
            .push(crate::profile::controllers::AxisBinding {
                code: 0,
                minimum: -100,
                maximum: 100,
                polarity: AxisPolarity::Bipolar,
            });
        profile.set_controllers(config).unwrap();
        profile.set_mapping(
            &InputAddress::Bound {
                device: DeviceId("evdev:v1:test".into()),
                input: InputId::Axis { index: 0 },
            },
            "Default",
            None,
            vec![Action::MapToVJoy {
                output: OutputAddress {
                    device: 99,
                    output: OutputId::Axis { id: VJoyAxis::X },
                },
            }],
        );
    };
    s.lock().rejected_axis_device = Some(99);
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![
        snapshot(false),
        InputUpdate::Frame(vec![button(true)]),
    ]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true)]);
    assert!(!s.lock().calls.iter().any(|call| call.starts_with("axis:")));
    assert_eq!(e.state.read().engine_status, EngineStatus::Running);
    assert_eq!(e.state.read().session.mapping_issues.len(), 1);
}

#[test]
fn resetting_shift_controller_releases_other_controllers_temporary_mode_outputs() {
    let (mut e, tx, s, _temp) = harness();
    let other = DeviceId("evdev:v1:other".into());
    let mut secondary = button(false);
    secondary.source = InputAddress::Bound {
        device: other.clone(),
        input: InputId::Button { index: 0 },
    };
    let mut profile = Profile::new(
        "test".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Temporary".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    let mut config = e
        .state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .controllers()
        .unwrap()
        .clone();
    config.selected.push(other.clone());
    config.bindings.push(DeviceBinding {
        observed_layout: None,
        device: other.clone(),
        axes: vec![],
        buttons: vec![704],
        hats: vec![],
        unavailable: vec![],
    });
    profile.set_controllers(config).unwrap();
    profile.set_mapping(
        &button(false).source,
        "Default",
        None,
        vec![Action::ChangeMode {
            strategy: crate::action::ModeChangeStrategy::Temporary {
                mode: "Temporary".into(),
            },
        }],
    );
    profile.set_mapping(
        &secondary.source,
        "Temporary",
        None,
        vec![Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Button { id: 2 },
            },
        }],
    );
    e.state.write().active_profile = Some(profile);
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![
        snapshot(false),
        InputUpdate::Snapshot {
            device: other.clone(),
            values: vec![secondary.clone()],
            recovered: false,
        },
        InputUpdate::Frame(vec![button(true)]),
    ]);
    e.tick().unwrap();
    assert_eq!(e.mode_state.current(), "Temporary");
    secondary.value = InputValue::Button { pressed: true };
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![secondary])]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(2, true)]);
    s.lock().polls.push_back(vec![InputUpdate::Reset {
        device: DeviceId("evdev:v1:test".into()),
    }]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(2, true), (2, false)]);
    assert_eq!(e.state.read().current_mode, "Default");
    assert_eq!(e.state.read().session.monitored, [other]);
    assert_eq!(e.state.read().engine_status, EngineStatus::Running);
}

#[test]
fn repeated_snapshot_releases_old_output_without_replaying_held_button() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![
        snapshot(false),
        InputUpdate::Frame(vec![button(true)]),
    ]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true)]);
    let generation = e.state.read().session.generation;
    s.lock().calls.clear();
    s.lock().polls.push_back(vec![snapshot(true)]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true), (1, false)]);
    assert!(e.state.read().session.generation > generation);
    assert!(
        !s.lock()
            .calls
            .iter()
            .any(|call| call == "release" || call == "acquire" || call == "stop")
    );
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![button(false)])]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true), (1, false)]);
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![button(true)])]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true), (1, false), (1, true)]);
}

#[test]
fn live_edit_held_primary_ignores_secondary_edges_until_primary_is_repressed() {
    let (mut e, tx, s, temp) = harness();
    e.state.write().profile_path = Some(temp.path().join("profile.toml"));
    {
        let mut state = e.state.write();
        let profile = state.active_profile.as_mut().unwrap();
        let mut config = profile.controllers().unwrap().clone();
        config.bindings[0].buttons.push(705);
        profile.set_controllers(config).unwrap();
    };
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![
        snapshot(false),
        InputUpdate::Frame(vec![button(true)]),
    ]);
    e.tick().unwrap();
    let mut secondary = button(true);
    secondary.source = InputAddress::Bound {
        device: DeviceId("evdev:v1:test".into()),
        input: InputId::Button { index: 1 },
    };
    tx.send(EngineCommand::SetMapping {
        input: button(false).source,
        mode: "Default".into(),
        name: None,
        actions: vec![Action::Conditional {
            condition: crate::action::Condition::ButtonPressed {
                input: secondary.source.clone(),
            },
            if_true: vec![Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Button { id: 2 },
                },
            }],
            if_false: vec![],
        }],
    })
    .unwrap();
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true), (1, false)]);
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![secondary])]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true), (1, false)]);
    s.lock()
        .polls
        .push_back(vec![InputUpdate::Frame(vec![button(false), button(true)])]);
    e.tick().unwrap();
    assert_eq!(s.lock().buttons, [(1, true), (1, false), (2, true)]);
}

#[test]
fn output_failure_still_consumes_other_controllers_updates_already_drained_in_poll() {
    for sampled in [false, true] {
        let (mut e, tx, s, _temp) = harness();
        tx.send(EngineCommand::Activate).unwrap();
        s.lock().polls.push_back(vec![snapshot(false)]);
        e.tick().unwrap();
        let other = DeviceId("evdev:v1:other".into());
        let mut event = button(true);
        event.source = InputAddress::Bound {
            device: other.clone(),
            input: InputId::Button { index: 0 },
        };
        let update = if sampled {
            InputUpdate::Snapshot {
                device: other.clone(),
                values: vec![event.clone()],
                recovered: false,
            }
        } else {
            InputUpdate::Frame(vec![event.clone()])
        };
        s.lock().failures.insert(Failure::Flush);
        s.lock()
            .polls
            .push_back(vec![InputUpdate::Frame(vec![button(true)]), update]);
        e.tick().unwrap_err();
        let state = e.state.read();
        assert_eq!(state.engine_status, EngineStatus::Faulted);
        assert!(state.session.captured.is_empty());
        assert!(state.session.monitored.contains(&other));
        assert!(
            state
                .input_cache
                .clone_compact()
                .iter()
                .any(|entry| entry.address == event.source && entry.value == event.value)
        );
    }
}

fn configure_axis(engine: &mut Engine, path: PathBuf) -> InputEvent {
    let event = InputEvent {
        source: InputAddress::Bound {
            device: DeviceId("evdev:v1:test".into()),
            input: InputId::Axis { index: 0 },
        },
        value: InputValue::Axis {
            value: AxisValue::raw(0.5),
            polarity: AxisPolarity::Bipolar,
        },
        timestamp: Instant::now(),
    };
    let mut state = engine.state.write();
    state.profile_path = Some(path);
    let profile = state.active_profile.as_mut().unwrap();
    let mut config = profile.controllers().unwrap().clone();
    config.bindings[0]
        .axes
        .push(crate::profile::controllers::AxisBinding {
            code: 0,
            minimum: -100,
            maximum: 100,
            polarity: AxisPolarity::Bipolar,
        });
    config.virtual_devices[0].axes = vec![VJoyAxis::X, VJoyAxis::Y];
    profile.set_controllers(config).unwrap();
    profile.set_mapping(
        &event.source,
        "Default",
        None,
        vec![Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        }],
    );
    event
}

#[test]
fn stationary_axis_mapping_edit_neutralizes_old_target_and_refreshes_processed_new_target() {
    let (mut e, tx, s, temp) = harness();
    let event = configure_axis(&mut e, temp.path().join("profile.toml"));
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![InputUpdate::Snapshot {
        device: DeviceId("evdev:v1:test".into()),
        values: vec![event.clone()],
        recovered: false,
    }]);
    e.tick().unwrap();
    assert_eq!(s.lock().axes, [(1, VJoyAxis::X, 0.5)]);
    s.lock().axes.clear();
    s.lock().calls.clear();
    tx.send(EngineCommand::SetMapping {
        input: event.source,
        mode: "Default".into(),
        name: None,
        actions: vec![
            Action::Invert,
            Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Axis { id: VJoyAxis::Y },
                },
            },
        ],
    })
    .unwrap();
    e.tick().unwrap();
    assert_eq!(
        s.lock().axes,
        [(1, VJoyAxis::X, 0.0), (1, VJoyAxis::Y, -0.5)]
    );
    assert!(
        !s.lock().calls.iter().any(|call| call == "release"
            || call == "acquire"
            || call == "start"
            || call == "stop")
    );
    s.lock().axes.clear();
    e.tick().unwrap();
    assert!(
        s.lock().axes.is_empty(),
        "stationary axis must not be written again without a change"
    );
}

#[test]
fn stationary_calibration_edit_refreshes_axis_once_without_double_calibration() {
    let (mut e, tx, s, temp) = harness();
    let event = configure_axis(&mut e, temp.path().join("profile.toml"));
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![InputUpdate::Snapshot {
        device: DeviceId("evdev:v1:test".into()),
        values: vec![event],
        recovered: false,
    }]);
    e.tick().unwrap();
    assert_eq!(s.lock().axes, [(1, VJoyAxis::X, 0.5)]);
    s.lock().axes.clear();
    tx.send(EngineCommand::SetCalibration {
        device: DeviceId("evdev:v1:test".into()),
        axis: 0,
        calibration: crate::processing::Calibration::new(-1.0, 0.0, 0.0, 2.0, true).unwrap(),
    })
    .unwrap();
    e.tick().unwrap();
    assert_eq!(s.lock().axes, [(1, VJoyAxis::X, 0.25)]);
    s.lock().axes.clear();
    e.tick().unwrap();
    assert!(s.lock().axes.is_empty());
}

#[test]
fn output_initialization_failure_preserves_preview_and_retry_activates() {
    let (mut e, tx, s, _temp) = harness();
    s.lock().start_failures_remaining = 1;
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(true)]);
    e.tick().unwrap();
    assert_eq!(e.state.read().engine_status, EngineStatus::Faulted);
    assert!(s.lock().acquired.is_empty());
    assert_eq!(
        e.state.read().input_cache.clone_compact()[0].value,
        InputValue::Button { pressed: true }
    );
    assert_eq!(
        e.state.read().session.monitored,
        [DeviceId("evdev:v1:test".into())]
    );
    tx.send(EngineCommand::Retry).unwrap();
    e.tick().unwrap();
    assert_eq!(e.state.read().engine_status, EngineStatus::Running);
    assert!(e.state.read().session.output_active);
    assert_eq!(s.lock().acquired, [vec![DeviceId("evdev:v1:test".into())]]);
    assert_eq!(
        s.lock()
            .calls
            .iter()
            .filter(|call| *call == "start")
            .count(),
        2
    );
    assert!(
        s.lock().buttons.is_empty(),
        "retry must not replay the held preview button"
    );
}

#[test]
fn stop_keeps_virtual_controllers_until_shutdown_and_restart_reuses_them() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    s.lock().polls.push_back(vec![snapshot(false)]);
    e.tick().unwrap();
    s.lock().calls.clear();
    tx.send(EngineCommand::Deactivate).unwrap();
    e.tick().unwrap();
    assert_eq!(e.read_status(), EngineStatus::Stopped);
    assert!(e.state.read().session.output_active);
    assert!(s.lock().calls.iter().any(|call| call == "neutral"));
    assert!(!s.lock().calls.iter().any(|call| call == "stop"));
    s.lock().calls.clear();
    tx.send(EngineCommand::Activate).unwrap();
    e.tick().unwrap();
    assert!(!s.lock().calls.iter().any(|call| call == "start"));
    tx.send(EngineCommand::Shutdown).unwrap();
    e.tick().unwrap();
    assert!(s.lock().calls.iter().any(|call| call == "stop"));
}

#[path = "output_config_tests.rs"]
mod output_config_tests;

#[path = "mapping_issue_tests.rs"]
mod mapping_issue_tests;

#[path = "preview_review_tests.rs"]
mod preview_review_tests;
