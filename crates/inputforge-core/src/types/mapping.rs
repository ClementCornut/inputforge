// Rust guideline compliant 2026-03-02

use std::{borrow::Cow, fmt};

use serde::{Deserialize, Serialize};

/// A keyboard key combination (key + modifiers).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    pub key: PhysicalKey,
    pub modifiers: Vec<KeyModifier>,
}

/// Physical keyboard key identified by its position, not by keyboard layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhysicalKey {
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Space,
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Insert,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    IntlBackslash,
    Semicolon,
    Quote,
    Backquote,
    Comma,
    Period,
    Slash,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    NumpadEnter,
}

/// Win32 Set 1 scan-code metadata for a physical key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalKeyScanCode {
    pub code: u16,
    pub extended: bool,
}

impl PhysicalKey {
    /// Return the Win32 Set 1 scan code for this physical key.
    #[must_use]
    pub const fn scan_code(self) -> PhysicalKeyScanCode {
        use PhysicalKey::{
            ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Backquote, Backslash, Backspace,
            BracketLeft, BracketRight, Comma, Delete, Digit0, Digit1, Digit2, Digit3, Digit4,
            Digit5, Digit6, Digit7, Digit8, Digit9, End, Enter, Equal, Escape, F1, F2, F3, F4, F5,
            F6, F7, F8, F9, F10, F11, F12, Home, Insert, IntlBackslash, KeyA, KeyB, KeyC, KeyD,
            KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR,
            KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Minus, Numpad0, Numpad1, Numpad2,
            Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd,
            NumpadDecimal, NumpadDivide, NumpadEnter, NumpadMultiply, NumpadSubtract, PageDown,
            PageUp, Period, Quote, Semicolon, Slash, Space, Tab,
        };

        let (code, extended) = match self {
            KeyA => (0x1e, false),
            KeyB => (0x30, false),
            KeyC => (0x2e, false),
            KeyD => (0x20, false),
            KeyE => (0x12, false),
            KeyF => (0x21, false),
            KeyG => (0x22, false),
            KeyH => (0x23, false),
            KeyI => (0x17, false),
            KeyJ => (0x24, false),
            KeyK => (0x25, false),
            KeyL => (0x26, false),
            KeyM => (0x32, false),
            KeyN => (0x31, false),
            KeyO => (0x18, false),
            KeyP => (0x19, false),
            KeyQ => (0x10, false),
            KeyR => (0x13, false),
            KeyS => (0x1f, false),
            KeyT => (0x14, false),
            KeyU => (0x16, false),
            KeyV => (0x2f, false),
            KeyW => (0x11, false),
            KeyX => (0x2d, false),
            KeyY => (0x15, false),
            KeyZ => (0x2c, false),
            Digit1 => (0x02, false),
            Digit2 => (0x03, false),
            Digit3 => (0x04, false),
            Digit4 => (0x05, false),
            Digit5 => (0x06, false),
            Digit6 => (0x07, false),
            Digit7 => (0x08, false),
            Digit8 => (0x09, false),
            Digit9 => (0x0a, false),
            Digit0 => (0x0b, false),
            F1 => (0x3b, false),
            F2 => (0x3c, false),
            F3 => (0x3d, false),
            F4 => (0x3e, false),
            F5 => (0x3f, false),
            F6 => (0x40, false),
            F7 => (0x41, false),
            F8 => (0x42, false),
            F9 => (0x43, false),
            F10 => (0x44, false),
            F11 => (0x57, false),
            F12 => (0x58, false),
            Space => (0x39, false),
            Enter => (0x1c, false),
            Tab => (0x0f, false),
            Escape => (0x01, false),
            Backspace => (0x0e, false),
            ArrowUp | Numpad8 => (0x48, matches!(self, ArrowUp)),
            ArrowDown | Numpad2 => (0x50, matches!(self, ArrowDown)),
            ArrowLeft | Numpad4 => (0x4b, matches!(self, ArrowLeft)),
            ArrowRight | Numpad6 => (0x4d, matches!(self, ArrowRight)),
            Home | Numpad7 => (0x47, matches!(self, Home)),
            End | Numpad1 => (0x4f, matches!(self, End)),
            PageUp | Numpad9 => (0x49, matches!(self, PageUp)),
            PageDown | Numpad3 => (0x51, matches!(self, PageDown)),
            Insert | Numpad0 => (0x52, matches!(self, Insert)),
            Delete | NumpadDecimal => (0x53, matches!(self, Delete)),
            Minus => (0x0c, false),
            Equal => (0x0d, false),
            BracketLeft => (0x1a, false),
            BracketRight => (0x1b, false),
            Backslash => (0x2b, false),
            IntlBackslash => (0x56, false),
            Semicolon => (0x27, false),
            Quote => (0x28, false),
            Backquote => (0x29, false),
            Comma => (0x33, false),
            Period => (0x34, false),
            Slash => (0x35, false),
            Numpad5 => (0x4c, false),
            NumpadAdd => (0x4e, false),
            NumpadSubtract => (0x4a, false),
            NumpadMultiply => (0x37, false),
            NumpadDivide => (0x35, true),
            NumpadEnter => (0x1c, true),
        };
        PhysicalKeyScanCode { code, extended }
    }

    /// Return the compact label shown in the GUI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        use PhysicalKey::{
            ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Backquote, Backslash, Backspace,
            BracketLeft, BracketRight, Comma, Delete, Digit0, Digit1, Digit2, Digit3, Digit4,
            Digit5, Digit6, Digit7, Digit8, Digit9, End, Enter, Equal, Escape, F1, F2, F3, F4, F5,
            F6, F7, F8, F9, F10, F11, F12, Home, Insert, IntlBackslash, KeyA, KeyB, KeyC, KeyD,
            KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR,
            KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Minus, Numpad0, Numpad1, Numpad2,
            Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd,
            NumpadDecimal, NumpadDivide, NumpadEnter, NumpadMultiply, NumpadSubtract, PageDown,
            PageUp, Period, Quote, Semicolon, Slash, Space, Tab,
        };
        match self {
            KeyA => "A",
            KeyB => "B",
            KeyC => "C",
            KeyD => "D",
            KeyE => "E",
            KeyF => "F",
            KeyG => "G",
            KeyH => "H",
            KeyI => "I",
            KeyJ => "J",
            KeyK => "K",
            KeyL => "L",
            KeyM => "M",
            KeyN => "N",
            KeyO => "O",
            KeyP => "P",
            KeyQ => "Q",
            KeyR => "R",
            KeyS => "S",
            KeyT => "T",
            KeyU => "U",
            KeyV => "V",
            KeyW => "W",
            KeyX => "X",
            KeyY => "Y",
            KeyZ => "Z",
            Digit0 => "0",
            Digit1 => "1",
            Digit2 => "2",
            Digit3 => "3",
            Digit4 => "4",
            Digit5 => "5",
            Digit6 => "6",
            Digit7 => "7",
            Digit8 => "8",
            Digit9 => "9",
            F1 => "F1",
            F2 => "F2",
            F3 => "F3",
            F4 => "F4",
            F5 => "F5",
            F6 => "F6",
            F7 => "F7",
            F8 => "F8",
            F9 => "F9",
            F10 => "F10",
            F11 => "F11",
            F12 => "F12",
            Space => "Space",
            Enter => "Enter",
            Tab => "Tab",
            Escape => "Esc",
            Backspace => "Backspace",
            Delete => "Delete",
            Insert => "Insert",
            ArrowUp => "Up",
            ArrowDown => "Down",
            ArrowLeft => "Left",
            ArrowRight => "Right",
            Home => "Home",
            End => "End",
            PageUp => "PageUp",
            PageDown => "PageDown",
            Minus => "Minus",
            Equal => "Equal",
            BracketLeft => "[",
            BracketRight => "]",
            Backslash => "\\",
            IntlBackslash => "< >",
            Semicolon => ";",
            Quote => "'",
            Backquote => "`",
            Comma => ",",
            Period => ".",
            Slash => "/",
            Numpad0 => "Num 0",
            Numpad1 => "Num 1",
            Numpad2 => "Num 2",
            Numpad3 => "Num 3",
            Numpad4 => "Num 4",
            Numpad5 => "Num 5",
            Numpad6 => "Num 6",
            Numpad7 => "Num 7",
            Numpad8 => "Num 8",
            Numpad9 => "Num 9",
            NumpadAdd => "Num +",
            NumpadSubtract => "Num -",
            NumpadMultiply => "Num *",
            NumpadDivide => "Num /",
            NumpadDecimal => "Num .",
            NumpadEnter => "Num Enter",
        }
    }

    /// Return the label shown in the GUI for the current platform layout.
    #[must_use]
    pub fn display_label(self) -> Cow<'static, str> {
        platform_display_label(self).unwrap_or_else(|| Cow::Borrowed(self.label()))
    }
}

#[cfg(all(target_os = "windows", feature = "win32-io"))]
fn platform_display_label(key: PhysicalKey) -> Option<Cow<'static, str>> {
    windows_layout_label(key).map(Cow::Owned)
}

#[cfg(not(all(target_os = "windows", feature = "win32-io")))]
fn platform_display_label(_key: PhysicalKey) -> Option<Cow<'static, str>> {
    None
}

#[cfg(all(target_os = "windows", feature = "win32-io"))]
#[expect(
    unsafe_code,
    reason = "Win32 keyboard layout translation uses User32 FFI"
)]
fn windows_layout_label(key: PhysicalKey) -> Option<String> {
    const EXTENDED_SCAN_CODE_PREFIX: u32 = 0xe000;
    const TO_UNICODE_NO_STATE_CHANGE: u32 = 0x04;

    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardLayout, MAPVK_VSC_TO_VK_EX, MapVirtualKeyExW, ToUnicodeEx,
    };

    let scan_code = key.scan_code();
    let hkl = unsafe { GetKeyboardLayout(0) };
    let scan = if scan_code.extended {
        u32::from(scan_code.code) | EXTENDED_SCAN_CODE_PREFIX
    } else {
        u32::from(scan_code.code)
    };
    let virtual_key = unsafe { MapVirtualKeyExW(scan, MAPVK_VSC_TO_VK_EX, Some(hkl)) };
    if virtual_key == 0 {
        return None;
    }

    let key_state = [0_u8; 256];
    let mut buffer = [0_u16; 8];
    let written = unsafe {
        ToUnicodeEx(
            virtual_key,
            scan,
            &key_state,
            &mut buffer,
            TO_UNICODE_NO_STATE_CHANGE,
            Some(hkl),
        )
    };
    if written <= 0 {
        return None;
    }

    let len = usize::try_from(written).ok()?.min(buffer.len());
    let label = String::from_utf16(&buffer[..len]).ok()?;
    normalize_layout_label(&label)
}

#[cfg(all(target_os = "windows", feature = "win32-io"))]
fn normalize_layout_label(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return None;
    }

    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if chars.next().is_none() && first.is_alphabetic() {
        Some(first.to_uppercase().collect())
    } else {
        Some(trimmed.to_owned())
    }
}

impl fmt::Display for PhysicalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Keyboard modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyModifier {
    Ctrl,
    Shift,
    Alt,
    Win,
}

/// Axis merge operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeOp {
    Bidirectional,
    Average,
    Maximum,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_combo_serde_roundtrip() {
        let combo = KeyCombo {
            key: PhysicalKey::IntlBackslash,
            modifiers: vec![KeyModifier::Ctrl, KeyModifier::Shift],
        };
        let json = serde_json::to_string(&combo).unwrap();
        let back: KeyCombo = serde_json::from_str(&json).unwrap();
        assert_eq!(combo, back);
    }

    #[test]
    fn physical_key_scan_codes_include_game_and_numpad_keys() {
        assert_eq!(
            PhysicalKey::KeyA.scan_code(),
            PhysicalKeyScanCode {
                code: 0x1e,
                extended: false,
            }
        );
        assert_eq!(
            PhysicalKey::Slash.scan_code(),
            PhysicalKeyScanCode {
                code: 0x35,
                extended: false,
            }
        );
        assert_eq!(
            PhysicalKey::NumpadDivide.scan_code(),
            PhysicalKeyScanCode {
                code: 0x35,
                extended: true,
            }
        );
        assert_eq!(
            PhysicalKey::NumpadEnter.scan_code(),
            PhysicalKeyScanCode {
                code: 0x1c,
                extended: true,
            }
        );
        assert_eq!(
            PhysicalKey::IntlBackslash.scan_code(),
            PhysicalKeyScanCode {
                code: 0x56,
                extended: false,
            }
        );
    }

    #[test]
    fn physical_key_labels_include_iso_key_fallback() {
        assert_eq!(PhysicalKey::IntlBackslash.label(), "< >");
    }

    #[test]
    fn physical_key_display_label_returns_a_visible_label() {
        let label = PhysicalKey::IntlBackslash.display_label();
        assert!(
            !label.trim().is_empty(),
            "layout-aware label must fall back to a static label"
        );
    }

    #[test]
    fn merge_op_all_variants() {
        let ops = [MergeOp::Bidirectional, MergeOp::Average, MergeOp::Maximum];
        assert_eq!(ops.len(), 3);
    }
}
