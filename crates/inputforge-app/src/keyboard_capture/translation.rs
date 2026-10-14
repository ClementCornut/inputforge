//! Preserve browser physical codes, locations and modifier flags.
use dioxus::prelude::{Code, Location};
use inputforge_core::types::{KeyModifier, PhysicalKey};
use serde::Deserialize;
use std::str::FromStr;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureKeyEventKind {
    KeyDown,
    KeyUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Browser keyboard events expose modifier state as independent booleans."
)]
pub(crate) struct CaptureKeyEvent {
    pub kind: CaptureKeyEventKind,
    pub code: Code,
    pub location: Location,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key_is_meta_or_super: bool,
    pub key_is_escape: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Browser keyboard events serialize modifier flags as independent booleans."
)]
pub(crate) struct BrowserKeyboardPayload {
    pub kind: String,
    pub key: String,
    pub code: String,
    pub location: u32,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl BrowserKeyboardPayload {
    pub(crate) fn to_capture_key_event(&self) -> CaptureKeyEvent {
        let code = Code::from_str(&self.code).unwrap_or(Code::Unidentified);
        CaptureKeyEvent {
            kind: match self.kind.as_str() {
                "keyup" => CaptureKeyEventKind::KeyUp,
                _ => CaptureKeyEventKind::KeyDown,
            },
            code,
            location: location_from_browser(self.location),
            ctrl: self.ctrl,
            alt: self.alt,
            shift: self.shift,
            meta: self.meta,
            key_is_meta_or_super: matches!(self.key.as_str(), "Meta" | "Super"),
            key_is_escape: self.key == "Escape" || matches!(code, Code::Escape),
        }
    }
}

const fn location_from_browser(location: u32) -> Location {
    match location {
        1 => Location::Left,
        2 => Location::Right,
        3 => Location::Numpad,
        _ => Location::Standard,
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

pub(super) const fn modifier_from_event(event: CaptureKeyEvent) -> Option<KeyModifier> {
    if let Some(modifier) = modifier_from_code(event.code) {
        return Some(modifier);
    }

    if !event.key_is_meta_or_super || !matches!(event.code, Code::Unidentified) {
        return None;
    }

    match event.location {
        Location::Left | Location::Standard => Some(KeyModifier::META_LEFT),
        Location::Right => Some(KeyModifier::META_RIGHT),
        Location::Numpad => None,
    }
}

pub(super) const fn physical_key_from_code(code: Code) -> Option<PhysicalKey> {
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
