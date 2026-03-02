// Rust guideline compliant 2026-03-03

use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_B, VK_BACK,
    VK_C, VK_D, VK_DELETE, VK_DOWN, VK_E, VK_END, VK_ESCAPE, VK_F, VK_F1, VK_F2, VK_F3, VK_F4,
    VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F16,
    VK_F17, VK_F18, VK_F19, VK_F20, VK_F21, VK_F22, VK_F23, VK_F24, VK_G, VK_H, VK_HOME, VK_I,
    VK_INSERT, VK_J, VK_K, VK_L, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_M, VK_N,
    VK_NEXT, VK_O, VK_P, VK_PRIOR, VK_Q, VK_R, VK_RETURN, VK_RIGHT, VK_S, VK_SPACE, VK_T, VK_TAB,
    VK_U, VK_UP, VK_V, VK_W, VK_X, VK_Y, VK_Z,
};

use crate::error::{EngineError, Result};
use crate::types::{KeyCombo, KeyModifier};

/// Keyboard output that simulates key presses via Win32 `SendInput`.
#[derive(Debug, Default)]
pub struct KeyboardOutput;

impl KeyboardOutput {
    /// Create a new `KeyboardOutput`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Send a key combination press or release.
    ///
    /// On press, modifiers are sent first (in order), then the main key.
    /// On release, the main key is released first, then modifiers in reverse.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] if the key name is unrecognized
    /// or if `SendInput` fails to inject the events.
    pub fn send_key(&self, combo: &KeyCombo, pressed: bool) -> Result<()> {
        let main_vk = key_to_vk(&combo.key).ok_or_else(|| EngineError::InvalidConfig {
            reason: format!("unrecognized key: {}", combo.key),
        })?;

        let mut inputs = Vec::with_capacity(combo.modifiers.len() + 1);

        if pressed {
            for modifier in &combo.modifiers {
                inputs.push(make_input(modifier_to_vk(*modifier), false));
            }
            inputs.push(make_input(main_vk, false));
        } else {
            inputs.push(make_input(main_vk, true));
            for modifier in combo.modifiers.iter().rev() {
                inputs.push(make_input(modifier_to_vk(*modifier), true));
            }
        }

        send_inputs(&inputs)
    }
}

/// Build a keyboard `INPUT` struct for a single key.
fn make_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Call Win32 `SendInput` with the given array of inputs.
///
/// # Errors
///
/// Returns [`EngineError::InvalidConfig`] if `SendInput` returns fewer
/// events than requested.
#[expect(unsafe_code, reason = "SendInput is a Win32 FFI call")]
fn send_inputs(inputs: &[INPUT]) -> Result<()> {
    if inputs.is_empty() {
        return Ok(());
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "size_of::<INPUT>() is 40 bytes, fits in i32"
    )]
    let cb_size = size_of::<INPUT>() as i32;

    // SAFETY: `inputs` is a valid slice of `INPUT` structs, and `cb_size`
    // matches the size of each element. `SendInput` reads from the slice
    // without retaining references.
    let sent = unsafe { SendInput(inputs, cb_size) };

    #[expect(
        clippy::cast_possible_truncation,
        reason = "inputs.len() is small (modifiers + 1 key)"
    )]
    let expected = inputs.len() as u32;

    if sent == expected {
        Ok(())
    } else {
        Err(EngineError::InvalidConfig {
            reason: format!("SendInput sent {sent}/{expected} events"),
        })
    }
}

/// Map a [`KeyModifier`] to the corresponding virtual key code.
fn modifier_to_vk(modifier: KeyModifier) -> VIRTUAL_KEY {
    match modifier {
        KeyModifier::Ctrl => VK_LCONTROL,
        KeyModifier::Shift => VK_LSHIFT,
        KeyModifier::Alt => VK_LMENU,
        KeyModifier::Win => VK_LWIN,
    }
}

/// Map a key name string to a Win32 virtual key code.
///
/// Supports: A-Z, 0-9, F1-F24, and common named keys.
/// Returns `None` for unrecognized key names.
fn key_to_vk(key: &str) -> Option<VIRTUAL_KEY> {
    // Single character keys: A-Z, 0-9.
    if key.len() == 1 {
        let ch = key.as_bytes()[0];
        return match ch {
            b'A'..=b'Z' | b'a'..=b'z' => Some(letter_to_vk(ch.to_ascii_uppercase())),
            b'0'..=b'9' => Some(digit_to_vk(ch)),
            _ => None,
        };
    }

    // Function keys: F1-F24.
    if let Some(num_str) = key.strip_prefix('F').or_else(|| key.strip_prefix('f')) {
        if let Ok(n) = num_str.parse::<u8>() {
            return fkey_to_vk(n);
        }
    }

    // Named keys (case-insensitive).
    let lower = key.to_ascii_lowercase();
    match lower.as_str() {
        "space" => Some(VK_SPACE),
        "enter" | "return" => Some(VK_RETURN),
        "tab" => Some(VK_TAB),
        "escape" | "esc" => Some(VK_ESCAPE),
        "backspace" => Some(VK_BACK),
        "delete" | "del" => Some(VK_DELETE),
        "insert" | "ins" => Some(VK_INSERT),
        "up" => Some(VK_UP),
        "down" => Some(VK_DOWN),
        "left" => Some(VK_LEFT),
        "right" => Some(VK_RIGHT),
        "home" => Some(VK_HOME),
        "end" => Some(VK_END),
        "pageup" | "pgup" => Some(VK_PRIOR),
        "pagedown" | "pgdn" => Some(VK_NEXT),
        _ => None,
    }
}

/// Map an ASCII uppercase letter (b'A'..=b'Z') to its virtual key code.
fn letter_to_vk(ch: u8) -> VIRTUAL_KEY {
    // VK_A through VK_Z are contiguous: VK_A = 0x41.
    let keys = [
        VK_A, VK_B, VK_C, VK_D, VK_E, VK_F, VK_G, VK_H, VK_I, VK_J, VK_K, VK_L, VK_M, VK_N, VK_O,
        VK_P, VK_Q, VK_R, VK_S, VK_T, VK_U, VK_V, VK_W, VK_X, VK_Y, VK_Z,
    ];
    keys[(ch - b'A') as usize]
}

/// Map an ASCII digit (b'0'..=b'9') to its virtual key code.
fn digit_to_vk(ch: u8) -> VIRTUAL_KEY {
    let digits = [VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9];
    digits[(ch - b'0') as usize]
}

/// Map a function key number (1-24) to its virtual key code.
fn fkey_to_vk(n: u8) -> Option<VIRTUAL_KEY> {
    let fkeys = [
        VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12,
        VK_F13, VK_F14, VK_F15, VK_F16, VK_F17, VK_F18, VK_F19, VK_F20, VK_F21, VK_F22, VK_F23,
        VK_F24,
    ];
    fkeys.get(usize::from(n.checked_sub(1)?)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_to_vk_letters() {
        assert_eq!(key_to_vk("A"), Some(VK_A));
        assert_eq!(key_to_vk("Z"), Some(VK_Z));
        assert_eq!(key_to_vk("a"), Some(VK_A));
        assert_eq!(key_to_vk("m"), Some(VK_M));
    }

    #[test]
    fn key_to_vk_digits() {
        assert_eq!(key_to_vk("0"), Some(VK_0));
        assert_eq!(key_to_vk("5"), Some(VK_5));
        assert_eq!(key_to_vk("9"), Some(VK_9));
    }

    #[test]
    fn key_to_vk_function_keys() {
        assert_eq!(key_to_vk("F1"), Some(VK_F1));
        assert_eq!(key_to_vk("F12"), Some(VK_F12));
        assert_eq!(key_to_vk("F24"), Some(VK_F24));
        assert_eq!(key_to_vk("f1"), Some(VK_F1));
    }

    #[test]
    fn key_to_vk_function_key_out_of_range() {
        assert_eq!(key_to_vk("F0"), None);
        assert_eq!(key_to_vk("F25"), None);
    }

    #[test]
    fn key_to_vk_named_keys() {
        assert_eq!(key_to_vk("Space"), Some(VK_SPACE));
        assert_eq!(key_to_vk("Enter"), Some(VK_RETURN));
        assert_eq!(key_to_vk("Return"), Some(VK_RETURN));
        assert_eq!(key_to_vk("Tab"), Some(VK_TAB));
        assert_eq!(key_to_vk("Escape"), Some(VK_ESCAPE));
        assert_eq!(key_to_vk("esc"), Some(VK_ESCAPE));
        assert_eq!(key_to_vk("Backspace"), Some(VK_BACK));
        assert_eq!(key_to_vk("Delete"), Some(VK_DELETE));
        assert_eq!(key_to_vk("del"), Some(VK_DELETE));
        assert_eq!(key_to_vk("Insert"), Some(VK_INSERT));
    }

    #[test]
    fn key_to_vk_arrow_keys() {
        assert_eq!(key_to_vk("Up"), Some(VK_UP));
        assert_eq!(key_to_vk("Down"), Some(VK_DOWN));
        assert_eq!(key_to_vk("Left"), Some(VK_LEFT));
        assert_eq!(key_to_vk("Right"), Some(VK_RIGHT));
    }

    #[test]
    fn key_to_vk_navigation_keys() {
        assert_eq!(key_to_vk("Home"), Some(VK_HOME));
        assert_eq!(key_to_vk("End"), Some(VK_END));
        assert_eq!(key_to_vk("PageUp"), Some(VK_PRIOR));
        assert_eq!(key_to_vk("PageDown"), Some(VK_NEXT));
        assert_eq!(key_to_vk("pgup"), Some(VK_PRIOR));
        assert_eq!(key_to_vk("pgdn"), Some(VK_NEXT));
    }

    #[test]
    fn key_to_vk_unknown_returns_none() {
        assert_eq!(key_to_vk("InvalidKey"), None);
        assert_eq!(key_to_vk(""), None);
        assert_eq!(key_to_vk("!"), None);
    }

    #[test]
    fn modifier_to_vk_all_variants() {
        assert_eq!(modifier_to_vk(KeyModifier::Ctrl), VK_LCONTROL);
        assert_eq!(modifier_to_vk(KeyModifier::Shift), VK_LSHIFT);
        assert_eq!(modifier_to_vk(KeyModifier::Alt), VK_LMENU);
        assert_eq!(modifier_to_vk(KeyModifier::Win), VK_LWIN);
    }

    #[test]
    #[expect(unsafe_code, reason = "accessing INPUT union field in test")]
    fn make_input_key_down_has_no_keyup_flag() {
        let input = make_input(VK_A, false);
        assert_eq!(input.r#type, INPUT_KEYBOARD);
        // SAFETY: we just created this as a keyboard input.
        let ki = unsafe { input.Anonymous.ki };
        assert_eq!(ki.wVk, VK_A);
        assert_eq!(ki.dwFlags, KEYBD_EVENT_FLAGS(0));
    }

    #[test]
    #[expect(unsafe_code, reason = "accessing INPUT union field in test")]
    fn make_input_key_up_has_keyup_flag() {
        let input = make_input(VK_A, true);
        // SAFETY: we just created this as a keyboard input.
        let ki = unsafe { input.Anonymous.ki };
        assert_eq!(ki.dwFlags, KEYEVENTF_KEYUP);
    }
}
