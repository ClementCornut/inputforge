// Rust guideline compliant 2026-05-13

use dioxus::prelude::Code;
use inputforge_core::types::{KeyCombo, KeyModifier, PhysicalKey};

const RELEASE_MODIFIER_HINT: &str = "Release the modifier to bind it";
const UNSUPPORTED_KEY_HINT: &str = "Unsupported key";
const MULTI_MODIFIER_ONLY_HINT: &str = "Modifier-only bindings must use one modifier";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CaptureKeyEventKind {
    KeyDown,
    KeyUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CaptureKeyEvent {
    pub kind: CaptureKeyEventKind,
    pub code: Code,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key_is_escape: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CaptureOutcome {
    Continue { hint: Option<&'static str> },
    Commit(KeyCombo),
    Cancel { hint: Option<&'static str> },
}

#[derive(Debug, Default)]
pub(super) struct KeyboardCapture {
    modifiers: Vec<KeyModifier>,
    cancelled: bool,
}

impl KeyboardCapture {
    pub(super) fn handle_event(&mut self, event: CaptureKeyEvent) -> CaptureOutcome {
        if self.cancelled {
            return CaptureOutcome::Cancel { hint: None };
        }
        if event.key_is_escape {
            self.cancelled = true;
            return CaptureOutcome::Cancel { hint: None };
        }

        match event.kind {
            CaptureKeyEventKind::KeyDown => self.handle_keydown(event),
            CaptureKeyEventKind::KeyUp => self.handle_keyup(event),
        }
    }

    fn handle_keydown(&mut self, event: CaptureKeyEvent) -> CaptureOutcome {
        if let Some(modifier) = modifier_from_code(event.code) {
            self.push_modifier(modifier);
            return CaptureOutcome::Continue {
                hint: Some(RELEASE_MODIFIER_HINT),
            };
        }

        let Some(key) = physical_key_from_code(event.code) else {
            self.cancelled = true;
            return CaptureOutcome::Cancel {
                hint: Some(UNSUPPORTED_KEY_HINT),
            };
        };

        let modifiers = self.modifiers_with_fallbacks(event);
        CaptureOutcome::Commit(KeyCombo { key, modifiers })
    }

    fn handle_keyup(&mut self, event: CaptureKeyEvent) -> CaptureOutcome {
        let Some(modifier) = modifier_from_code(event.code) else {
            return CaptureOutcome::Continue { hint: None };
        };

        if self.modifiers.len() > 1 {
            self.cancelled = true;
            return CaptureOutcome::Cancel {
                hint: Some(MULTI_MODIFIER_ONLY_HINT),
            };
        }

        if self.modifiers.first().copied() == Some(modifier) {
            return CaptureOutcome::Commit(KeyCombo {
                key: modifier.physical_key(),
                modifiers: Vec::new(),
            });
        }

        CaptureOutcome::Continue { hint: None }
    }

    fn push_modifier(&mut self, modifier: KeyModifier) {
        if modifier == KeyModifier::ALT_RIGHT {
            self.modifiers
                .retain(|stored| *stored != KeyModifier::CONTROL_LEFT);
        } else if modifier == KeyModifier::CONTROL_LEFT
            && self.modifiers.contains(&KeyModifier::ALT_RIGHT)
        {
            return;
        }

        if !self.modifiers.contains(&modifier) {
            self.modifiers.push(modifier);
        }
    }

    fn modifiers_with_fallbacks(&self, event: CaptureKeyEvent) -> Vec<KeyModifier> {
        let mut modifiers = self.modifiers.clone();
        if event.ctrl && !modifiers.contains(&KeyModifier::ALT_RIGHT) {
            push_unique(&mut modifiers, KeyModifier::CONTROL_LEFT);
        }
        if event.alt {
            push_unique(&mut modifiers, KeyModifier::ALT_LEFT);
        }
        if event.shift {
            push_unique(&mut modifiers, KeyModifier::SHIFT_LEFT);
        }
        if event.meta {
            push_unique(&mut modifiers, KeyModifier::META_LEFT);
        }
        modifiers
    }
}

fn push_unique(modifiers: &mut Vec<KeyModifier>, modifier: KeyModifier) {
    if !modifiers.contains(&modifier) {
        modifiers.push(modifier);
    }
}

const fn modifier_from_code(code: Code) -> Option<KeyModifier> {
    match code {
        Code::ControlLeft => Some(KeyModifier::CONTROL_LEFT),
        Code::ControlRight => Some(KeyModifier::CONTROL_RIGHT),
        Code::ShiftLeft => Some(KeyModifier::SHIFT_LEFT),
        Code::ShiftRight => Some(KeyModifier::SHIFT_RIGHT),
        Code::AltLeft => Some(KeyModifier::ALT_LEFT),
        Code::AltRight => Some(KeyModifier::ALT_RIGHT),
        Code::MetaLeft => Some(KeyModifier::META_LEFT),
        Code::MetaRight => Some(KeyModifier::META_RIGHT),
        _ => None,
    }
}

const fn physical_key_from_code(code: Code) -> Option<PhysicalKey> {
    match code {
        Code::KeyA => Some(PhysicalKey::KeyA),
        Code::KeyB => Some(PhysicalKey::KeyB),
        Code::KeyC => Some(PhysicalKey::KeyC),
        Code::KeyD => Some(PhysicalKey::KeyD),
        Code::KeyE => Some(PhysicalKey::KeyE),
        Code::KeyF => Some(PhysicalKey::KeyF),
        Code::KeyG => Some(PhysicalKey::KeyG),
        Code::KeyH => Some(PhysicalKey::KeyH),
        Code::KeyI => Some(PhysicalKey::KeyI),
        Code::KeyJ => Some(PhysicalKey::KeyJ),
        Code::KeyK => Some(PhysicalKey::KeyK),
        Code::KeyL => Some(PhysicalKey::KeyL),
        Code::KeyM => Some(PhysicalKey::KeyM),
        Code::KeyN => Some(PhysicalKey::KeyN),
        Code::KeyO => Some(PhysicalKey::KeyO),
        Code::KeyP => Some(PhysicalKey::KeyP),
        Code::KeyQ => Some(PhysicalKey::KeyQ),
        Code::KeyR => Some(PhysicalKey::KeyR),
        Code::KeyS => Some(PhysicalKey::KeyS),
        Code::KeyT => Some(PhysicalKey::KeyT),
        Code::KeyU => Some(PhysicalKey::KeyU),
        Code::KeyV => Some(PhysicalKey::KeyV),
        Code::KeyW => Some(PhysicalKey::KeyW),
        Code::KeyX => Some(PhysicalKey::KeyX),
        Code::KeyY => Some(PhysicalKey::KeyY),
        Code::KeyZ => Some(PhysicalKey::KeyZ),
        Code::Digit0 => Some(PhysicalKey::Digit0),
        Code::Digit1 => Some(PhysicalKey::Digit1),
        Code::Digit2 => Some(PhysicalKey::Digit2),
        Code::Digit3 => Some(PhysicalKey::Digit3),
        Code::Digit4 => Some(PhysicalKey::Digit4),
        Code::Digit5 => Some(PhysicalKey::Digit5),
        Code::Digit6 => Some(PhysicalKey::Digit6),
        Code::Digit7 => Some(PhysicalKey::Digit7),
        Code::Digit8 => Some(PhysicalKey::Digit8),
        Code::Digit9 => Some(PhysicalKey::Digit9),
        Code::F1 => Some(PhysicalKey::F1),
        Code::F2 => Some(PhysicalKey::F2),
        Code::F3 => Some(PhysicalKey::F3),
        Code::F4 => Some(PhysicalKey::F4),
        Code::F5 => Some(PhysicalKey::F5),
        Code::F6 => Some(PhysicalKey::F6),
        Code::F7 => Some(PhysicalKey::F7),
        Code::F8 => Some(PhysicalKey::F8),
        Code::F9 => Some(PhysicalKey::F9),
        Code::F10 => Some(PhysicalKey::F10),
        Code::F11 => Some(PhysicalKey::F11),
        Code::F12 => Some(PhysicalKey::F12),
        Code::Space => Some(PhysicalKey::Space),
        Code::Enter => Some(PhysicalKey::Enter),
        Code::Tab => Some(PhysicalKey::Tab),
        Code::Escape => Some(PhysicalKey::Escape),
        Code::Backspace => Some(PhysicalKey::Backspace),
        Code::Delete => Some(PhysicalKey::Delete),
        Code::Insert => Some(PhysicalKey::Insert),
        Code::ArrowUp => Some(PhysicalKey::ArrowUp),
        Code::ArrowDown => Some(PhysicalKey::ArrowDown),
        Code::ArrowLeft => Some(PhysicalKey::ArrowLeft),
        Code::ArrowRight => Some(PhysicalKey::ArrowRight),
        Code::Home => Some(PhysicalKey::Home),
        Code::End => Some(PhysicalKey::End),
        Code::PageUp => Some(PhysicalKey::PageUp),
        Code::PageDown => Some(PhysicalKey::PageDown),
        Code::Minus => Some(PhysicalKey::Minus),
        Code::Equal => Some(PhysicalKey::Equal),
        Code::BracketLeft => Some(PhysicalKey::BracketLeft),
        Code::BracketRight => Some(PhysicalKey::BracketRight),
        Code::Backslash => Some(PhysicalKey::Backslash),
        Code::IntlBackslash => Some(PhysicalKey::IntlBackslash),
        Code::Semicolon => Some(PhysicalKey::Semicolon),
        Code::Quote => Some(PhysicalKey::Quote),
        Code::Backquote => Some(PhysicalKey::Backquote),
        Code::Comma => Some(PhysicalKey::Comma),
        Code::Period => Some(PhysicalKey::Period),
        Code::Slash => Some(PhysicalKey::Slash),
        Code::Numpad0 => Some(PhysicalKey::Numpad0),
        Code::Numpad1 => Some(PhysicalKey::Numpad1),
        Code::Numpad2 => Some(PhysicalKey::Numpad2),
        Code::Numpad3 => Some(PhysicalKey::Numpad3),
        Code::Numpad4 => Some(PhysicalKey::Numpad4),
        Code::Numpad5 => Some(PhysicalKey::Numpad5),
        Code::Numpad6 => Some(PhysicalKey::Numpad6),
        Code::Numpad7 => Some(PhysicalKey::Numpad7),
        Code::Numpad8 => Some(PhysicalKey::Numpad8),
        Code::Numpad9 => Some(PhysicalKey::Numpad9),
        Code::NumpadAdd => Some(PhysicalKey::NumpadAdd),
        Code::NumpadSubtract => Some(PhysicalKey::NumpadSubtract),
        Code::NumpadMultiply => Some(PhysicalKey::NumpadMultiply),
        Code::NumpadDivide => Some(PhysicalKey::NumpadDivide),
        Code::NumpadDecimal => Some(PhysicalKey::NumpadDecimal),
        Code::NumpadEnter => Some(PhysicalKey::NumpadEnter),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            key_is_escape: matches!(code, Code::Escape),
        }
    }

    #[test]
    fn modifier_only_commits_on_keyup() {
        let mut capture = KeyboardCapture::default();

        assert_eq!(
            capture.handle_event(keydown(Code::ControlRight)),
            CaptureOutcome::Continue {
                hint: Some("Release the modifier to bind it")
            }
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
}
