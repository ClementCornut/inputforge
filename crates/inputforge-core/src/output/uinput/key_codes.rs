use crate::types::{KeyCombo, PhysicalKey};
use evdev::KeyCode;

macro_rules! define_key_codes {
    ($($key:ident => $code:ident),+ $(,)?) => {
        pub(super) const fn key_code(key: PhysicalKey) -> u16 {
            match key {
                $(PhysicalKey::$key => KeyCode::$code.0),+
            }
        }

        const ALL_KEYS: &[PhysicalKey] = &[$(PhysicalKey::$key),+];
    };
}

define_key_codes! {
    ControlLeft => KEY_LEFTCTRL,
    ControlRight => KEY_RIGHTCTRL,
    ShiftLeft => KEY_LEFTSHIFT,
    ShiftRight => KEY_RIGHTSHIFT,
    AltLeft => KEY_LEFTALT,
    AltRight => KEY_RIGHTALT,
    MetaLeft => KEY_LEFTMETA,
    MetaRight => KEY_RIGHTMETA,
    KeyA => KEY_A,
    KeyB => KEY_B,
    KeyC => KEY_C,
    KeyD => KEY_D,
    KeyE => KEY_E,
    KeyF => KEY_F,
    KeyG => KEY_G,
    KeyH => KEY_H,
    KeyI => KEY_I,
    KeyJ => KEY_J,
    KeyK => KEY_K,
    KeyL => KEY_L,
    KeyM => KEY_M,
    KeyN => KEY_N,
    KeyO => KEY_O,
    KeyP => KEY_P,
    KeyQ => KEY_Q,
    KeyR => KEY_R,
    KeyS => KEY_S,
    KeyT => KEY_T,
    KeyU => KEY_U,
    KeyV => KEY_V,
    KeyW => KEY_W,
    KeyX => KEY_X,
    KeyY => KEY_Y,
    KeyZ => KEY_Z,
    Digit0 => KEY_0,
    Digit1 => KEY_1,
    Digit2 => KEY_2,
    Digit3 => KEY_3,
    Digit4 => KEY_4,
    Digit5 => KEY_5,
    Digit6 => KEY_6,
    Digit7 => KEY_7,
    Digit8 => KEY_8,
    Digit9 => KEY_9,
    F1 => KEY_F1,
    F2 => KEY_F2,
    F3 => KEY_F3,
    F4 => KEY_F4,
    F5 => KEY_F5,
    F6 => KEY_F6,
    F7 => KEY_F7,
    F8 => KEY_F8,
    F9 => KEY_F9,
    F10 => KEY_F10,
    F11 => KEY_F11,
    F12 => KEY_F12,
    Space => KEY_SPACE,
    Enter => KEY_ENTER,
    Tab => KEY_TAB,
    Escape => KEY_ESC,
    Backspace => KEY_BACKSPACE,
    Delete => KEY_DELETE,
    Insert => KEY_INSERT,
    ArrowUp => KEY_UP,
    ArrowDown => KEY_DOWN,
    ArrowLeft => KEY_LEFT,
    ArrowRight => KEY_RIGHT,
    Home => KEY_HOME,
    End => KEY_END,
    PageUp => KEY_PAGEUP,
    PageDown => KEY_PAGEDOWN,
    Minus => KEY_MINUS,
    Equal => KEY_EQUAL,
    BracketLeft => KEY_LEFTBRACE,
    BracketRight => KEY_RIGHTBRACE,
    Backslash => KEY_BACKSLASH,
    IntlBackslash => KEY_102ND,
    Semicolon => KEY_SEMICOLON,
    Quote => KEY_APOSTROPHE,
    Backquote => KEY_GRAVE,
    Comma => KEY_COMMA,
    Period => KEY_DOT,
    Slash => KEY_SLASH,
    Numpad0 => KEY_KP0,
    Numpad1 => KEY_KP1,
    Numpad2 => KEY_KP2,
    Numpad3 => KEY_KP3,
    Numpad4 => KEY_KP4,
    Numpad5 => KEY_KP5,
    Numpad6 => KEY_KP6,
    Numpad7 => KEY_KP7,
    Numpad8 => KEY_KP8,
    Numpad9 => KEY_KP9,
    NumpadAdd => KEY_KPPLUS,
    NumpadSubtract => KEY_KPMINUS,
    NumpadMultiply => KEY_KPASTERISK,
    NumpadDivide => KEY_KPSLASH,
    NumpadDecimal => KEY_KPDOT,
    NumpadEnter => KEY_KPENTER,
}

pub(super) fn all_codes() -> Vec<u16> {
    ALL_KEYS.iter().copied().map(key_code).collect()
}

pub(super) fn combo_codes(combo: &KeyCombo) -> Vec<u16> {
    let mut codes = Vec::with_capacity(combo.modifiers.len() + 1);
    for modifier in &combo.modifiers {
        let code = key_code(modifier.physical_key());
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    let base = key_code(combo.key);
    if !codes.contains(&base) {
        codes.push(base);
    }
    codes
}
