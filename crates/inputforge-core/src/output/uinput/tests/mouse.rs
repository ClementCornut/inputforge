use super::{super::device::DeviceKey, fixtures};
use crate::{
    action::MouseTarget,
    error::EngineError,
    output::{MouseSink, OutputKind, OutputPhase},
};
use evdev::{KeyCode, RelativeAxisCode};
use std::io;

#[test]
fn mouse_buttons_and_vertical_wheel_use_exact_linux_codes() {
    let (mut mouse, world) = fixtures::started_mouse();
    let buttons = [
        (MouseTarget::LeftButton, KeyCode::BTN_LEFT.0),
        (MouseTarget::RightButton, KeyCode::BTN_RIGHT.0),
        (MouseTarget::MiddleButton, KeyCode::BTN_MIDDLE.0),
        (MouseTarget::BackButton, KeyCode::BTN_SIDE.0),
        (MouseTarget::ForwardButton, KeyCode::BTN_EXTRA.0),
    ];
    for (target, _) in buttons {
        mouse.button_down(target).unwrap();
        mouse.button_up(target).unwrap();
    }
    mouse.wheel(MouseTarget::WheelUp).unwrap();
    mouse.wheel(MouseTarget::WheelDown).unwrap();

    let events: Vec<_> = world
        .lock()
        .unwrap()
        .emitted
        .iter()
        .flat_map(|(_, events)| events)
        .filter(|event| event.event_type().0 != 0)
        .map(|event| (event.event_type().0, event.code(), event.value()))
        .collect();
    let expected: Vec<_> = buttons
        .into_iter()
        .flat_map(|(_, code)| [(1, code, 1), (1, code, 0)])
        .chain([
            (2, RelativeAxisCode::REL_WHEEL.0, 1),
            (2, RelativeAxisCode::REL_WHEEL.0, -1),
        ])
        .collect();
    assert_eq!(events, expected);
}

#[test]
fn invalid_button_and_wheel_combinations_are_rejected() {
    let (mut mouse, _) = fixtures::started_mouse();
    assert!(matches!(
        mouse.button_down(MouseTarget::WheelUp),
        Err(EngineError::InvalidConfig { .. })
    ));
    assert!(matches!(
        mouse.button_up(MouseTarget::WheelDown),
        Err(EngineError::InvalidConfig { .. })
    ));
    assert!(matches!(
        mouse.wheel(MouseTarget::LeftButton),
        Err(EngineError::InvalidConfig { .. })
    ));
}

#[test]
fn mouse_wrapper_reports_typed_runtime_failures() {
    let (mut mouse, world) = fixtures::started_mouse();
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::BrokenPipe.into()));
    let EngineError::InjectionFailed { failure } = mouse.wheel(MouseTarget::WheelUp).unwrap_err()
    else {
        panic!("expected typed injection failure");
    };
    assert_eq!(failure.output, OutputKind::Mouse);
    assert_eq!(failure.phase, OutputPhase::Emission);
    assert_eq!(failure.category, io::ErrorKind::BrokenPipe);
    assert!(failure.details.contains("write event packet"));
    assert!(failure.cleanup.is_empty());
    assert!(world.lock().unwrap().alive.contains(&DeviceKey::Mouse));
}
