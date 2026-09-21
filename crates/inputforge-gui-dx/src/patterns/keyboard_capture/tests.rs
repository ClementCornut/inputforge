use dioxus::prelude::{Code, Location};
use inputforge_core::types::{KeyCombo, KeyModifier, PhysicalKey};

use super::{
    BrowserKeyboardPayload, CaptureDelivery, CaptureKeyEvent, CaptureKeyEventKind, CaptureOutcome,
    KeyboardCaptureCore, KeyboardCaptureOwner, KeyboardCaptureUpdate,
};

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
fn azerty_character_keeps_its_physical_key_code() {
    let mut core = KeyboardCaptureCore::default();
    let owner = KeyboardCaptureOwner(8);
    let _ = core.start(owner);

    assert_eq!(
        core.handle_payload(&BrowserKeyboardPayload {
            kind: "keydown".to_owned(),
            key: "a".to_owned(),
            code: "KeyQ".to_owned(),
            location: 0,
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }),
        Some(KeyboardCaptureUpdate {
            owner,
            outcome: CaptureOutcome::Commit(KeyCombo {
                key: PhysicalKey::KeyQ,
                modifiers: Vec::new(),
            }),
        })
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
fn core_cancels_previous_owner_when_new_owner_starts() {
    let mut core = KeyboardCaptureCore::default();
    let first = KeyboardCaptureOwner(41);
    let second = KeyboardCaptureOwner(42);

    assert_eq!(core.start(first), None);
    assert_eq!(core.start(second), Some(first));
    assert_eq!(core.active_owner(), Some(second));
}

#[test]
fn core_routes_commit_to_active_owner_and_clears_capture() {
    let mut core = KeyboardCaptureCore::default();
    let owner = KeyboardCaptureOwner(7);
    let _ = core.start(owner);

    assert_eq!(
        core.handle_payload(&BrowserKeyboardPayload {
            kind: "keydown".to_owned(),
            key: "Shift".to_owned(),
            code: "ShiftRight".to_owned(),
            location: 2,
            ctrl: false,
            alt: false,
            shift: true,
            meta: false,
        }),
        Some(KeyboardCaptureUpdate {
            owner,
            outcome: CaptureOutcome::Continue { hint: None },
        })
    );
    assert_eq!(
        core.handle_payload(&BrowserKeyboardPayload {
            kind: "keydown".to_owned(),
            key: "J".to_owned(),
            code: "KeyJ".to_owned(),
            location: 0,
            ctrl: false,
            alt: false,
            shift: true,
            meta: false,
        }),
        Some(KeyboardCaptureUpdate {
            owner,
            outcome: CaptureOutcome::Commit(KeyCombo {
                key: PhysicalKey::KeyJ,
                modifiers: vec![KeyModifier::SHIFT_RIGHT],
            }),
        })
    );
    assert_eq!(core.active_owner(), None);
}

#[test]
fn core_cancels_active_owner_and_ignores_later_payloads() {
    let mut core = KeyboardCaptureCore::default();
    let owner = KeyboardCaptureOwner(11);

    assert_eq!(core.start(owner), None);
    assert!(core.cancel(owner));
    assert_eq!(core.active_owner(), None);
    assert_eq!(
        core.handle_payload(&BrowserKeyboardPayload {
            kind: "keydown".to_owned(),
            key: "J".to_owned(),
            code: "KeyJ".to_owned(),
            location: 0,
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }),
        None
    );
}

#[test]
fn core_commits_alt_f_and_alt_g() {
    for (code, key) in [("KeyF", PhysicalKey::KeyF), ("KeyG", PhysicalKey::KeyG)] {
        let mut core = KeyboardCaptureCore::default();
        let owner = KeyboardCaptureOwner(17);
        let _ = core.start(owner);

        assert_eq!(
            core.handle_payload(&BrowserKeyboardPayload {
                kind: "keydown".to_owned(),
                key: "Alt".to_owned(),
                code: "AltLeft".to_owned(),
                location: 1,
                ctrl: false,
                alt: true,
                shift: false,
                meta: false,
            }),
            Some(KeyboardCaptureUpdate {
                owner,
                outcome: CaptureOutcome::Continue { hint: None },
            })
        );
        assert_eq!(
            core.handle_payload(&BrowserKeyboardPayload {
                kind: "keydown".to_owned(),
                key: code.trim_start_matches("Key").to_owned(),
                code: code.to_owned(),
                location: 0,
                ctrl: false,
                alt: true,
                shift: false,
                meta: false,
            }),
            Some(KeyboardCaptureUpdate {
                owner,
                outcome: CaptureOutcome::Commit(KeyCombo {
                    key,
                    modifiers: vec![KeyModifier::ALT_LEFT],
                }),
            })
        );
    }
}

#[test]
fn observed_alt_keydown_commits_base_keyup_when_keydown_is_swallowed() {
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

    assert_eq!(
        capture.handle_event(keyup(Code::KeyG)),
        CaptureOutcome::Continue { hint: None }
    );
}

#[test]
fn capture_delivery_sequence_increments() {
    let first = CaptureDelivery::new(
        1,
        KeyboardCaptureUpdate {
            owner: KeyboardCaptureOwner(1),
            outcome: CaptureOutcome::Continue { hint: None },
        },
    );
    let second = first.next(KeyboardCaptureUpdate {
        owner: KeyboardCaptureOwner(1),
        outcome: CaptureOutcome::Cancel { hint: None },
    });

    assert_eq!(first.sequence, 1);
    assert_eq!(second.sequence, 2);
}

#[test]
fn listener_script_is_passive_until_browser_flag_is_armed() {
    assert!(
        super::KEYBOARD_CAPTURE_LISTENER_JS.contains("window.__inputforgeKeyboardCaptureArmed")
    );
    assert!(
        super::KEYBOARD_CAPTURE_LISTENER_JS
            .contains("if (!window.__inputforgeKeyboardCaptureArmed) return;")
    );
    assert!(
        super::KEYBOARD_CAPTURE_LISTENER_JS
            .contains("window.addEventListener('keydown', keydown, true);")
    );
    assert!(
        super::KEYBOARD_CAPTURE_LISTENER_JS
            .contains("window.addEventListener('keyup', keyup, true);")
    );
}

#[test]
fn modifier_only_commits_on_keyup() {
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

    assert_eq!(
        capture.handle_event(keydown(Code::Unidentified)),
        CaptureOutcome::Cancel {
            hint: Some("Unsupported key")
        }
    );
}

#[test]
fn observed_right_alt_suppresses_left_alt_and_synthetic_control_fallbacks() {
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

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
    let mut capture = super::KeyboardCapture::default();

    let _ = capture.handle_event(keydown(Code::ControlLeft));
    let _ = capture.handle_event(keydown(Code::ShiftLeft));

    assert_eq!(
        capture.handle_event(keyup(Code::ShiftLeft)),
        CaptureOutcome::Cancel {
            hint: Some("Modifier-only bindings must use one modifier")
        }
    );
}
