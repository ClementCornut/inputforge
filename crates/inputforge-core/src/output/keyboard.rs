// Rust guideline compliant 2026-03-03

use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY,
};

use crate::error::{EngineError, Result};
use crate::types::{KeyCombo, KeyModifier, PhysicalKeyScanCode};

use super::traits::KeyboardSink;

/// Maximum number of `INPUT` events per key combination.
///
/// This accounts for the 4 possible modifier keys plus 1 main key.
const MAX_COMBO_INPUTS: usize = 5;

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
    /// Returns [`EngineError::OutputFailed`] if `SendInput` fails to inject the events.
    pub fn send_key(&self, combo: &KeyCombo, pressed: bool) -> Result<()> {
        let (inputs, count) = build_key_inputs(combo, pressed);
        send_inputs(&inputs[..count])
    }
}

impl KeyboardSink for KeyboardOutput {
    fn key_down(&mut self, combo: &KeyCombo) -> Result<()> {
        Self::send_key(&*self, combo, true)
    }

    fn key_up(&mut self, combo: &KeyCombo) -> Result<()> {
        Self::send_key(&*self, combo, false)
    }
}

/// Build a keyboard `INPUT` struct for a single key.
///
/// Uses scan-code input (`KEYEVENTF_SCANCODE`) so physical positions remain
/// stable across keyboard layouts. Extended keys add the E0 prefix flag.
fn make_input(scan_code: PhysicalKeyScanCode, key_up: bool) -> INPUT {
    let mut flags = KEYEVENTF_SCANCODE;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    if scan_code.extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scan_code.code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn build_key_inputs(combo: &KeyCombo, pressed: bool) -> ([INPUT; MAX_COMBO_INPUTS], usize) {
    let mut inputs = [INPUT::default(); MAX_COMBO_INPUTS];
    let mut count = 0;

    if pressed {
        for modifier in &combo.modifiers {
            inputs[count] = make_input(modifier_to_scan_code(*modifier), false);
            count += 1;
        }
        inputs[count] = make_input(combo.key.scan_code(), false);
        count += 1;
    } else {
        inputs[count] = make_input(combo.key.scan_code(), true);
        count += 1;
        for modifier in combo.modifiers.iter().rev() {
            inputs[count] = make_input(modifier_to_scan_code(*modifier), true);
            count += 1;
        }
    }

    (inputs, count)
}

/// Call Win32 `SendInput` with the given array of inputs.
///
/// # Errors
///
/// Returns [`EngineError::OutputFailed`] if `SendInput` returns fewer
/// events than requested.
#[expect(unsafe_code, reason = "SendInput and GetLastError are Win32 FFI calls")]
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
        // SAFETY: `GetLastError` is a stateless Win32 call with no preconditions.
        let last_err = unsafe { GetLastError() };
        Err(EngineError::OutputFailed {
            reason: format!("SendInput sent {sent}/{expected} events (last error: {last_err:?})"),
        })
    }
}

/// Maps a key modifier to its corresponding left-side scan code.
///
/// All modifiers are sent as the left-hand variant. Left Windows uses an
/// extended scan code; Ctrl, Shift, and Alt do not.
fn modifier_to_scan_code(modifier: KeyModifier) -> PhysicalKeyScanCode {
    match modifier {
        KeyModifier::Ctrl => PhysicalKeyScanCode {
            code: 0x1d,
            extended: false,
        },
        KeyModifier::Shift => PhysicalKeyScanCode {
            code: 0x2a,
            extended: false,
        },
        KeyModifier::Alt => PhysicalKeyScanCode {
            code: 0x38,
            extended: false,
        },
        KeyModifier::Win => PhysicalKeyScanCode {
            code: 0x5b,
            extended: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PhysicalKey;
    use windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS;

    #[expect(unsafe_code, reason = "accessing INPUT union field in test")]
    fn keyboard_input_parts(input: INPUT) -> (VIRTUAL_KEY, u16, KEYBD_EVENT_FLAGS) {
        // SAFETY: tests only pass values produced by `make_input`.
        let ki = unsafe { input.Anonymous.ki };
        (ki.wVk, ki.wScan, ki.dwFlags)
    }

    #[test]
    fn modifier_to_scan_code_all_variants() {
        assert_eq!(
            modifier_to_scan_code(KeyModifier::Ctrl),
            PhysicalKeyScanCode {
                code: 0x1d,
                extended: false,
            }
        );
        assert_eq!(
            modifier_to_scan_code(KeyModifier::Shift),
            PhysicalKeyScanCode {
                code: 0x2a,
                extended: false,
            }
        );
        assert_eq!(
            modifier_to_scan_code(KeyModifier::Alt),
            PhysicalKeyScanCode {
                code: 0x38,
                extended: false,
            }
        );
        assert_eq!(
            modifier_to_scan_code(KeyModifier::Win),
            PhysicalKeyScanCode {
                code: 0x5b,
                extended: true,
            }
        );
    }

    #[test]
    fn make_input_key_down_uses_physical_scan_code() {
        let input = make_input(PhysicalKey::KeyA.scan_code(), false);
        assert_eq!(input.r#type, INPUT_KEYBOARD);
        let (vk, scan, flags) = keyboard_input_parts(input);
        assert_eq!(vk, VIRTUAL_KEY(0));
        assert_eq!(scan, 0x1e);
        assert_eq!(flags, KEYEVENTF_SCANCODE);
    }

    #[test]
    fn make_input_numpad_divide_uses_extended_scan_code() {
        let input = make_input(PhysicalKey::NumpadDivide.scan_code(), false);
        let (vk, scan, flags) = keyboard_input_parts(input);
        assert_eq!(vk, VIRTUAL_KEY(0));
        assert_eq!(scan, 0x35);
        assert_eq!(flags, KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY);
    }

    #[test]
    fn make_input_key_up_has_keyup_flag() {
        let input = make_input(
            PhysicalKeyScanCode {
                code: 0x1e,
                extended: false,
            },
            true,
        );
        let (vk, scan, flags) = keyboard_input_parts(input);
        assert_eq!(vk, VIRTUAL_KEY(0));
        assert_eq!(scan, 0x1e);
        assert_eq!(flags, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);
    }

    #[test]
    fn make_input_extended_key_down_has_extended_flag() {
        let input = make_input(PhysicalKey::ArrowUp.scan_code(), false);
        let (_, _, flags) = keyboard_input_parts(input);
        assert_eq!(flags, KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY);
    }

    #[test]
    fn make_input_extended_key_up_has_both_flags() {
        let input = make_input(PhysicalKey::ArrowUp.scan_code(), true);
        let (_, _, flags) = keyboard_input_parts(input);
        assert_eq!(
            flags,
            KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY
        );
    }

    #[test]
    fn build_key_inputs_presses_modifiers_before_key() {
        let combo = KeyCombo {
            key: PhysicalKey::NumpadEnter,
            modifiers: vec![KeyModifier::Ctrl, KeyModifier::Shift],
        };

        let (inputs, count) = build_key_inputs(&combo, true);

        assert_eq!(count, 3);
        assert_eq!(
            keyboard_input_parts(inputs[0]),
            (VIRTUAL_KEY(0), 0x1d, KEYEVENTF_SCANCODE)
        );
        assert_eq!(
            keyboard_input_parts(inputs[1]),
            (VIRTUAL_KEY(0), 0x2a, KEYEVENTF_SCANCODE)
        );
        assert_eq!(
            keyboard_input_parts(inputs[2]),
            (
                VIRTUAL_KEY(0),
                0x1c,
                KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY
            )
        );
    }

    #[test]
    fn build_key_inputs_releases_key_before_modifiers_in_reverse_order() {
        let combo = KeyCombo {
            key: PhysicalKey::NumpadDivide,
            modifiers: vec![KeyModifier::Ctrl, KeyModifier::Alt],
        };

        let (inputs, count) = build_key_inputs(&combo, false);

        assert_eq!(count, 3);
        assert_eq!(
            keyboard_input_parts(inputs[0]),
            (
                VIRTUAL_KEY(0),
                0x35,
                KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY
            )
        );
        assert_eq!(
            keyboard_input_parts(inputs[1]),
            (VIRTUAL_KEY(0), 0x38, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP)
        );
        assert_eq!(
            keyboard_input_parts(inputs[2]),
            (VIRTUAL_KEY(0), 0x1d, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP)
        );
    }
}
