use super::{super::key_codes, fixtures};
use crate::{
    error::EngineError,
    output::{KeyboardSink, OutputKind, OutputPhase},
    types::{KeyCombo, KeyModifier, PhysicalKey},
};
use evdev::KeyCode;
use std::io;

use PhysicalKey::*;

const EXPECTED_KEY_CODES: &[(PhysicalKey, KeyCode)] = &[
    (ControlLeft, KeyCode::KEY_LEFTCTRL),
    (ControlRight, KeyCode::KEY_RIGHTCTRL),
    (ShiftLeft, KeyCode::KEY_LEFTSHIFT),
    (ShiftRight, KeyCode::KEY_RIGHTSHIFT),
    (AltLeft, KeyCode::KEY_LEFTALT),
    (AltRight, KeyCode::KEY_RIGHTALT),
    (MetaLeft, KeyCode::KEY_LEFTMETA),
    (MetaRight, KeyCode::KEY_RIGHTMETA),
    (KeyA, KeyCode::KEY_A),
    (KeyB, KeyCode::KEY_B),
    (KeyC, KeyCode::KEY_C),
    (KeyD, KeyCode::KEY_D),
    (KeyE, KeyCode::KEY_E),
    (KeyF, KeyCode::KEY_F),
    (KeyG, KeyCode::KEY_G),
    (KeyH, KeyCode::KEY_H),
    (KeyI, KeyCode::KEY_I),
    (KeyJ, KeyCode::KEY_J),
    (KeyK, KeyCode::KEY_K),
    (KeyL, KeyCode::KEY_L),
    (KeyM, KeyCode::KEY_M),
    (KeyN, KeyCode::KEY_N),
    (KeyO, KeyCode::KEY_O),
    (KeyP, KeyCode::KEY_P),
    (KeyQ, KeyCode::KEY_Q),
    (KeyR, KeyCode::KEY_R),
    (KeyS, KeyCode::KEY_S),
    (KeyT, KeyCode::KEY_T),
    (KeyU, KeyCode::KEY_U),
    (KeyV, KeyCode::KEY_V),
    (KeyW, KeyCode::KEY_W),
    (KeyX, KeyCode::KEY_X),
    (KeyY, KeyCode::KEY_Y),
    (KeyZ, KeyCode::KEY_Z),
    (Digit0, KeyCode::KEY_0),
    (Digit1, KeyCode::KEY_1),
    (Digit2, KeyCode::KEY_2),
    (Digit3, KeyCode::KEY_3),
    (Digit4, KeyCode::KEY_4),
    (Digit5, KeyCode::KEY_5),
    (Digit6, KeyCode::KEY_6),
    (Digit7, KeyCode::KEY_7),
    (Digit8, KeyCode::KEY_8),
    (Digit9, KeyCode::KEY_9),
    (F1, KeyCode::KEY_F1),
    (F2, KeyCode::KEY_F2),
    (F3, KeyCode::KEY_F3),
    (F4, KeyCode::KEY_F4),
    (F5, KeyCode::KEY_F5),
    (F6, KeyCode::KEY_F6),
    (F7, KeyCode::KEY_F7),
    (F8, KeyCode::KEY_F8),
    (F9, KeyCode::KEY_F9),
    (F10, KeyCode::KEY_F10),
    (F11, KeyCode::KEY_F11),
    (F12, KeyCode::KEY_F12),
    (Space, KeyCode::KEY_SPACE),
    (Enter, KeyCode::KEY_ENTER),
    (Tab, KeyCode::KEY_TAB),
    (Escape, KeyCode::KEY_ESC),
    (Backspace, KeyCode::KEY_BACKSPACE),
    (Delete, KeyCode::KEY_DELETE),
    (Insert, KeyCode::KEY_INSERT),
    (ArrowUp, KeyCode::KEY_UP),
    (ArrowDown, KeyCode::KEY_DOWN),
    (ArrowLeft, KeyCode::KEY_LEFT),
    (ArrowRight, KeyCode::KEY_RIGHT),
    (Home, KeyCode::KEY_HOME),
    (End, KeyCode::KEY_END),
    (PageUp, KeyCode::KEY_PAGEUP),
    (PageDown, KeyCode::KEY_PAGEDOWN),
    (Minus, KeyCode::KEY_MINUS),
    (Equal, KeyCode::KEY_EQUAL),
    (BracketLeft, KeyCode::KEY_LEFTBRACE),
    (BracketRight, KeyCode::KEY_RIGHTBRACE),
    (Backslash, KeyCode::KEY_BACKSLASH),
    (IntlBackslash, KeyCode::KEY_102ND),
    (Semicolon, KeyCode::KEY_SEMICOLON),
    (Quote, KeyCode::KEY_APOSTROPHE),
    (Backquote, KeyCode::KEY_GRAVE),
    (Comma, KeyCode::KEY_COMMA),
    (Period, KeyCode::KEY_DOT),
    (Slash, KeyCode::KEY_SLASH),
    (Numpad0, KeyCode::KEY_KP0),
    (Numpad1, KeyCode::KEY_KP1),
    (Numpad2, KeyCode::KEY_KP2),
    (Numpad3, KeyCode::KEY_KP3),
    (Numpad4, KeyCode::KEY_KP4),
    (Numpad5, KeyCode::KEY_KP5),
    (Numpad6, KeyCode::KEY_KP6),
    (Numpad7, KeyCode::KEY_KP7),
    (Numpad8, KeyCode::KEY_KP8),
    (Numpad9, KeyCode::KEY_KP9),
    (NumpadAdd, KeyCode::KEY_KPPLUS),
    (NumpadSubtract, KeyCode::KEY_KPMINUS),
    (NumpadMultiply, KeyCode::KEY_KPASTERISK),
    (NumpadDivide, KeyCode::KEY_KPSLASH),
    (NumpadDecimal, KeyCode::KEY_KPDOT),
    (NumpadEnter, KeyCode::KEY_KPENTER),
];

#[test]
fn every_physical_key_has_the_expected_linux_code() {
    for &(key, code) in EXPECTED_KEY_CODES {
        assert_eq!(key_codes::key_code(key), code.0, "{key:?}");
    }
    assert_eq!(
        key_codes::all_codes(),
        EXPECTED_KEY_CODES
            .iter()
            .map(|(_, code)| code.0)
            .collect::<Vec<_>>()
    );
}

#[test]
fn combination_encoding_preserves_order_deduplicates_and_keeps_base_last() {
    let combo = KeyCombo {
        key: KeyA,
        modifiers: vec![
            KeyModifier::SHIFT_LEFT,
            KeyModifier::CONTROL_LEFT,
            KeyModifier::SHIFT_LEFT,
        ],
    };
    assert_eq!(
        key_codes::combo_codes(&combo),
        [
            KeyCode::KEY_LEFTSHIFT.0,
            KeyCode::KEY_LEFTCTRL.0,
            KeyCode::KEY_A.0
        ]
    );

    let base_is_modifier = KeyCombo {
        key: ShiftLeft,
        modifiers: vec![KeyModifier::SHIFT_LEFT, KeyModifier::CONTROL_LEFT],
    };
    assert_eq!(
        key_codes::combo_codes(&base_is_modifier),
        [KeyCode::KEY_LEFTSHIFT.0, KeyCode::KEY_LEFTCTRL.0]
    );
}

#[test]
fn keyboard_wrapper_reports_typed_native_failures_and_cleanup() {
    let (mut keyboard, world) = fixtures::keyboard();
    world.lock().unwrap().fail_create = Some(super::super::device::DeviceKey::Keyboard);
    let EngineError::InjectionFailed { failure } = keyboard.start().unwrap_err() else {
        panic!("expected typed injection failure");
    };
    assert_eq!(failure.output, OutputKind::Keyboard);
    assert_eq!(failure.phase, OutputPhase::Initialization);
    assert_eq!(failure.category, io::ErrorKind::PermissionDenied);
    assert!(failure.details.contains("create uinput"));
    assert!(failure.cleanup.is_empty());
}

#[test]
fn keyboard_readiness_timeout_preserves_the_native_error_category() {
    let (mut keyboard, world) = fixtures::keyboard();
    world.lock().unwrap().fail_ready = Some((
        super::super::device::DeviceKey::Keyboard,
        io::ErrorKind::PermissionDenied,
    ));

    let EngineError::InjectionFailed { failure } = keyboard.start().unwrap_err() else {
        panic!("expected typed injection failure");
    };
    assert_eq!(failure.phase, OutputPhase::Readiness);
    assert_eq!(failure.category, io::ErrorKind::PermissionDenied);
    assert!(failure.details.contains("output readiness deadline"));
    assert!(!failure.details.contains("event node"));
}

#[test]
fn keyboard_cleanup_failure_keeps_secondary_native_evidence_separate() {
    let (mut keyboard, world) = fixtures::keyboard();
    keyboard.start().unwrap();
    keyboard
        .key_down(&KeyCombo {
            key: KeyA,
            modifiers: vec![],
        })
        .unwrap();
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::BrokenPipe.into()));
    world
        .lock()
        .unwrap()
        .fail_destroy
        .insert(super::super::device::DeviceKey::Keyboard);

    let EngineError::InjectionFailed { failure } = keyboard.stop().unwrap_err() else {
        panic!("expected typed injection failure");
    };
    assert_eq!(failure.phase, OutputPhase::Release);
    assert_eq!(failure.category, io::ErrorKind::BrokenPipe);
    assert!(failure.details.contains("write event packet"));
    assert_eq!(failure.cleanup.len(), 1);
    assert!(failure.cleanup[0].contains("destroy output"));
}
