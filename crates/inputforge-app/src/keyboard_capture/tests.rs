use dioxus::prelude::{Code, Location};
use inputforge_core::types::{KeyCombo, KeyModifier, PhysicalKey};

use super::machine::{CaptureOutcome, KeyboardCapture};
use super::translation::{BrowserKeyboardPayload, CaptureKeyEvent, CaptureKeyEventKind};

fn keydown(code: Code) -> CaptureKeyEvent {
    event(CaptureKeyEventKind::KeyDown, code)
}

fn keyup(code: Code) -> CaptureKeyEvent {
    event(CaptureKeyEventKind::KeyUp, code)
}

fn event(kind: CaptureKeyEventKind, code: Code) -> CaptureKeyEvent {
    CaptureKeyEvent {
        kind,
        code,
        location: Location::Standard,
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
        key_is_meta_or_super: false,
        key_is_escape: matches!(code, Code::Escape),
    }
}

fn unidentified_meta_keydown(location: Location) -> CaptureKeyEvent {
    CaptureKeyEvent {
        code: Code::Unidentified,
        location,
        key_is_meta_or_super: true,
        ..keydown(Code::Unidentified)
    }
}

fn unidentified_meta_keyup(location: Location) -> CaptureKeyEvent {
    CaptureKeyEvent {
        code: Code::Unidentified,
        location,
        key_is_meta_or_super: true,
        ..keyup(Code::Unidentified)
    }
}

fn keydown_with_modifiers(code: Code, pressed_modifiers: &[KeyModifier]) -> CaptureKeyEvent {
    CaptureKeyEvent {
        ctrl: pressed_modifiers.contains(&KeyModifier::CONTROL_LEFT)
            || pressed_modifiers.contains(&KeyModifier::CONTROL_RIGHT),
        alt: pressed_modifiers.contains(&KeyModifier::ALT_LEFT)
            || pressed_modifiers.contains(&KeyModifier::ALT_RIGHT),
        shift: pressed_modifiers.contains(&KeyModifier::SHIFT_LEFT)
            || pressed_modifiers.contains(&KeyModifier::SHIFT_RIGHT),
        meta: pressed_modifiers.contains(&KeyModifier::META_LEFT)
            || pressed_modifiers.contains(&KeyModifier::META_RIGHT),
        ..keydown(code)
    }
}

fn keyup_with_modifiers(code: Code, pressed_modifiers: &[KeyModifier]) -> CaptureKeyEvent {
    CaptureKeyEvent {
        ctrl: pressed_modifiers.contains(&KeyModifier::CONTROL_LEFT)
            || pressed_modifiers.contains(&KeyModifier::CONTROL_RIGHT),
        alt: pressed_modifiers.contains(&KeyModifier::ALT_LEFT)
            || pressed_modifiers.contains(&KeyModifier::ALT_RIGHT),
        shift: pressed_modifiers.contains(&KeyModifier::SHIFT_LEFT)
            || pressed_modifiers.contains(&KeyModifier::SHIFT_RIGHT),
        meta: pressed_modifiers.contains(&KeyModifier::META_LEFT)
            || pressed_modifiers.contains(&KeyModifier::META_RIGHT),
        ..keyup(code)
    }
}

#[test]
fn browser_payload_maps_to_capture_key_event() {
    let payload = BrowserKeyboardPayload {
        kind: "keydown".to_owned(),
        key: "J".to_owned(),
        code: "KeyJ".to_owned(),
        location: 0,
        ctrl: true,
        alt: false,
        shift: true,
        meta: false,
    };

    assert_eq!(
        payload.to_capture_key_event(),
        CaptureKeyEvent {
            kind: CaptureKeyEventKind::KeyDown,
            code: Code::KeyJ,
            location: Location::Standard,
            ctrl: true,
            alt: false,
            shift: true,
            meta: false,
            key_is_meta_or_super: false,
            key_is_escape: false,
        }
    );
}

#[test]
fn browser_payload_maps_escape_to_cancel_event() {
    let payload = BrowserKeyboardPayload {
        kind: "keydown".to_owned(),
        key: "Escape".to_owned(),
        code: "Escape".to_owned(),
        location: 0,
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
    };

    let event = payload.to_capture_key_event();
    assert_eq!(event.code, Code::Escape);
    assert!(event.key_is_escape);
}

#[test]
fn browser_payload_maps_unidentified_right_super() {
    let payload = BrowserKeyboardPayload {
        kind: "keydown".to_owned(),
        key: "Super".to_owned(),
        code: "Unidentified".to_owned(),
        location: 2,
        ctrl: false,
        alt: false,
        shift: false,
        meta: true,
    };

    assert_eq!(
        payload.to_capture_key_event(),
        CaptureKeyEvent {
            kind: CaptureKeyEventKind::KeyDown,
            code: Code::Unidentified,
            location: Location::Right,
            ctrl: false,
            alt: false,
            shift: false,
            meta: true,
            key_is_meta_or_super: true,
            key_is_escape: false,
        }
    );
}

#[test]
fn observed_alt_keydown_commits_base_keyup_when_keydown_is_swallowed() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::AltLeft));

    assert_eq!(
        capture.handle_event(keyup_with_modifiers(Code::KeyG, &[KeyModifier::ALT_LEFT])),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyG,
            modifiers: vec![KeyModifier::ALT_LEFT],
        })
    );
}

#[test]
fn unmodified_base_keyup_does_not_commit() {
    let mut capture = KeyboardCapture::default();

    assert_eq!(
        capture.handle_event(keyup(Code::KeyG)),
        CaptureOutcome::Continue { hint: None }
    );
}

#[test]
fn modifier_only_commits_on_keyup() {
    let mut capture = KeyboardCapture::default();

    assert_eq!(
        capture.handle_event(keydown(Code::ControlRight)),
        CaptureOutcome::Continue { hint: None }
    );
    assert_eq!(
        capture.handle_event(keyup(Code::ControlRight)),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::ControlRight,
            modifiers: Vec::new(),
        })
    );
}

#[test]
fn combo_commits_with_physical_right_modifier() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::ShiftRight));

    assert_eq!(
        capture.handle_event(keydown(Code::KeyA)),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyA,
            modifiers: vec![KeyModifier::SHIFT_RIGHT],
        })
    );
}

#[test]
fn observed_right_shift_suppresses_left_shift_fallback() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::ShiftRight));

    assert_eq!(
        capture.handle_event(keydown_with_modifiers(
            Code::KeyJ,
            &[KeyModifier::SHIFT_RIGHT],
        )),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyJ,
            modifiers: vec![KeyModifier::SHIFT_RIGHT],
        })
    );
}

#[test]
fn observed_right_control_suppresses_left_control_fallback() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::ControlRight));

    assert_eq!(
        capture.handle_event(keydown_with_modifiers(
            Code::KeyJ,
            &[KeyModifier::CONTROL_RIGHT],
        )),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyJ,
            modifiers: vec![KeyModifier::CONTROL_RIGHT],
        })
    );
}

#[test]
fn observed_right_meta_suppresses_left_meta_fallback() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::MetaRight));

    assert_eq!(
        capture.handle_event(keydown_with_modifiers(
            Code::KeyJ,
            &[KeyModifier::META_RIGHT],
        )),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyJ,
            modifiers: vec![KeyModifier::META_RIGHT],
        })
    );
}

#[test]
fn unidentified_left_meta_modifier_commits_on_keyup() {
    let mut capture = KeyboardCapture::default();

    assert_eq!(
        capture.handle_event(unidentified_meta_keydown(Location::Left)),
        CaptureOutcome::Continue { hint: None }
    );
    assert_eq!(
        capture.handle_event(unidentified_meta_keyup(Location::Left)),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::MetaLeft,
            modifiers: Vec::new(),
        })
    );
}

#[test]
fn unidentified_right_super_modifier_commits_with_base_key() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(unidentified_meta_keydown(Location::Right));

    assert_eq!(
        capture.handle_event(keydown_with_modifiers(
            Code::KeyJ,
            &[KeyModifier::META_RIGHT],
        )),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyJ,
            modifiers: vec![KeyModifier::META_RIGHT],
        })
    );
}

#[test]
fn unidentified_non_meta_key_is_unsupported() {
    let mut capture = KeyboardCapture::default();

    assert_eq!(
        capture.handle_event(keydown(Code::Unidentified)),
        CaptureOutcome::Cancel {
            hint: Some("Unsupported key")
        }
    );
}

#[test]
fn observed_right_alt_suppresses_left_alt_and_synthetic_control_fallbacks() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::AltRight));

    assert_eq!(
        capture.handle_event(keydown_with_modifiers(
            Code::KeyJ,
            &[KeyModifier::ALT_RIGHT, KeyModifier::CONTROL_LEFT],
        )),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyJ,
            modifiers: vec![KeyModifier::ALT_RIGHT],
        })
    );
}

#[test]
fn unobserved_shift_uses_left_shift_fallback() {
    let mut capture = KeyboardCapture::default();

    assert_eq!(
        capture.handle_event(keydown_with_modifiers(
            Code::KeyJ,
            &[KeyModifier::SHIFT_LEFT],
        )),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyJ,
            modifiers: vec![KeyModifier::SHIFT_LEFT],
        })
    );
}

#[test]
fn alt_right_suppresses_synthetic_control_left() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::ControlLeft));
    let _ = capture.handle_event(keydown(Code::AltRight));

    assert_eq!(
        capture.handle_event(keydown(Code::KeyA)),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyA,
            modifiers: vec![KeyModifier::ALT_RIGHT],
        })
    );
}

#[test]
fn unsupported_key_after_modifier_cancels_without_keyup_commit() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::ControlLeft));
    assert_eq!(
        capture.handle_event(keydown(Code::AudioVolumeUp)),
        CaptureOutcome::Cancel {
            hint: Some("Unsupported key")
        }
    );
    assert_eq!(
        capture.handle_event(keyup(Code::ControlLeft)),
        CaptureOutcome::Cancel { hint: None }
    );
}

#[test]
fn multiple_modifier_only_presses_do_not_commit() {
    let mut capture = KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::ControlLeft));
    let _ = capture.handle_event(keydown(Code::ShiftLeft));

    assert_eq!(
        capture.handle_event(keyup(Code::ShiftLeft)),
        CaptureOutcome::Cancel {
            hint: Some("Modifier-only bindings must use one modifier")
        }
    );
}

#[test]
fn main_and_numpad_enter_keep_separate_physical_identities() {
    for (code, location, key) in [
        ("Enter", 0, PhysicalKey::Enter),
        ("NumpadEnter", 3, PhysicalKey::NumpadEnter),
    ] {
        let payload = BrowserKeyboardPayload {
            kind: "keydown".into(),
            key: "Enter".into(),
            code: code.into(),
            location,
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        };
        assert_eq!(
            KeyboardCapture::default().handle_event(payload.to_capture_key_event()),
            CaptureOutcome::Commit(KeyCombo {
                key,
                modifiers: vec![]
            })
        );
    }
}
#[test]
fn repeated_modifier_keydown_keeps_one_physical_modifier() {
    let mut capture = KeyboardCapture::default();
    let _ = capture.handle_event(keydown(Code::ShiftRight));
    let _ = capture.handle_event(keydown(Code::ShiftRight));
    assert_eq!(
        capture.handle_event(keydown(Code::KeyJ)),
        CaptureOutcome::Commit(KeyCombo {
            key: PhysicalKey::KeyJ,
            modifiers: vec![KeyModifier::SHIFT_RIGHT]
        })
    );
}
