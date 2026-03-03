// Rust guideline compliant 2026-03-03

use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, VIRTUAL_KEY, VK_BACK, VK_DELETE,
    VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9,
    VK_F10, VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F16, VK_F17, VK_F18, VK_F19, VK_F20, VK_F21,
    VK_F22, VK_F23, VK_F24, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_NEXT, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RWIN, VK_SPACE, VK_TAB,
    VK_UP,
};

use crate::error::{EngineError, Result};
use crate::types::{KeyCombo, KeyModifier};

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
    /// Returns [`EngineError::InvalidConfig`] if the key name is unrecognized,
    /// or [`EngineError::OutputFailed`] if `SendInput` fails to inject the events.
    pub fn send_key(&self, combo: &KeyCombo, pressed: bool) -> Result<()> {
        let main_vk = key_to_vk(&combo.key).ok_or_else(|| EngineError::InvalidConfig {
            reason: format!("unrecognized key: {}", combo.key),
        })?;

        let mut inputs = [INPUT::default(); MAX_COMBO_INPUTS];
        let mut count = 0;

        if pressed {
            for modifier in &combo.modifiers {
                inputs[count] = make_input(modifier_to_vk(*modifier), false);
                count += 1;
            }
            inputs[count] = make_input(main_vk, false);
            count += 1;
        } else {
            inputs[count] = make_input(main_vk, true);
            count += 1;
            for modifier in combo.modifiers.iter().rev() {
                inputs[count] = make_input(modifier_to_vk(*modifier), true);
                count += 1;
            }
        }

        send_inputs(&inputs[..count])
    }
}

impl KeyboardSink for KeyboardOutput {
    fn send_key(&mut self, combo: &KeyCombo) -> Result<()> {
        // Press then release the key combination.
        // Use explicit inherent method calls to avoid resolving to the
        // trait method (which has a different signature).
        Self::send_key(&*self, combo, true)?;
        Self::send_key(&*self, combo, false)
    }
}

/// Returns `true` for virtual key codes that require the extended-key flag.
///
/// Extended keys include navigation keys (arrows, Home/End, PageUp/Down,
/// Insert, Delete) and right-side modifier keys. Without this flag,
/// Windows may route scan codes to the numpad cluster instead.
fn is_extended_key(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_UP
            | VK_DOWN
            | VK_LEFT
            | VK_RIGHT
            | VK_HOME
            | VK_END
            | VK_PRIOR
            | VK_NEXT
            | VK_INSERT
            | VK_DELETE
            | VK_RCONTROL
            | VK_RMENU
            | VK_RWIN
    )
}

/// Build a keyboard `INPUT` struct for a single key.
///
/// Maps the virtual key code to its hardware scan code via
/// [`MapVirtualKeyW`] so that applications reading scan codes
/// (common in games) receive correct input. For extended keys
/// (arrows, navigation, right-side modifiers), the
/// `KEYEVENTF_EXTENDEDKEY` flag is set automatically.
fn make_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let mut flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };

    if is_extended_key(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    #[expect(
        unsafe_code,
        reason = "MapVirtualKeyW is an unsafe FFI call in the windows crate"
    )]
    #[expect(clippy::cast_possible_truncation, reason = "scan codes fit in u16")]
    // SAFETY: `MapVirtualKeyW` is a stateless Win32 lookup that converts a
    // virtual-key code to a scan code. It has no preconditions beyond valid
    // parameter types, which are satisfied by the `VIRTUAL_KEY` newtype.
    let scan = unsafe { MapVirtualKeyW(u32::from(vk.0), MAPVK_VK_TO_VSC) } as u16;

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
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

/// Maps a key modifier to its corresponding left-side virtual key code.
///
/// All modifiers are sent as the left-hand variant (e.g., `VK_LCONTROL`
/// rather than `VK_RCONTROL`).
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

    // Named keys (case-insensitive, without allocating).
    if key.eq_ignore_ascii_case("space") {
        return Some(VK_SPACE);
    }
    if key.eq_ignore_ascii_case("enter") || key.eq_ignore_ascii_case("return") {
        return Some(VK_RETURN);
    }
    if key.eq_ignore_ascii_case("tab") {
        return Some(VK_TAB);
    }
    if key.eq_ignore_ascii_case("escape") || key.eq_ignore_ascii_case("esc") {
        return Some(VK_ESCAPE);
    }
    if key.eq_ignore_ascii_case("backspace") {
        return Some(VK_BACK);
    }
    if key.eq_ignore_ascii_case("delete") || key.eq_ignore_ascii_case("del") {
        return Some(VK_DELETE);
    }
    if key.eq_ignore_ascii_case("insert") || key.eq_ignore_ascii_case("ins") {
        return Some(VK_INSERT);
    }
    if key.eq_ignore_ascii_case("up") {
        return Some(VK_UP);
    }
    if key.eq_ignore_ascii_case("down") {
        return Some(VK_DOWN);
    }
    if key.eq_ignore_ascii_case("left") {
        return Some(VK_LEFT);
    }
    if key.eq_ignore_ascii_case("right") {
        return Some(VK_RIGHT);
    }
    if key.eq_ignore_ascii_case("home") {
        return Some(VK_HOME);
    }
    if key.eq_ignore_ascii_case("end") {
        return Some(VK_END);
    }
    if key.eq_ignore_ascii_case("pageup") || key.eq_ignore_ascii_case("pgup") {
        return Some(VK_PRIOR);
    }
    if key.eq_ignore_ascii_case("pagedown") || key.eq_ignore_ascii_case("pgdn") {
        return Some(VK_NEXT);
    }

    None
}

/// Map an ASCII uppercase letter (b'A'..=b'Z') to its virtual key code.
fn letter_to_vk(ch: u8) -> VIRTUAL_KEY {
    // VK_A through VK_Z equal ASCII 0x41-0x5A.
    VIRTUAL_KEY(u16::from(ch))
}

/// Map an ASCII digit (b'0'..=b'9') to its virtual key code.
fn digit_to_vk(ch: u8) -> VIRTUAL_KEY {
    // VK_0 through VK_9 equal ASCII 0x30-0x39.
    VIRTUAL_KEY(u16::from(ch))
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

    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_0, VK_5, VK_9, VK_A, VK_M, VK_Z};

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
    fn is_extended_key_navigation_keys() {
        assert!(is_extended_key(VK_UP));
        assert!(is_extended_key(VK_DOWN));
        assert!(is_extended_key(VK_LEFT));
        assert!(is_extended_key(VK_RIGHT));
        assert!(is_extended_key(VK_HOME));
        assert!(is_extended_key(VK_END));
        assert!(is_extended_key(VK_PRIOR));
        assert!(is_extended_key(VK_NEXT));
        assert!(is_extended_key(VK_INSERT));
        assert!(is_extended_key(VK_DELETE));
    }

    #[test]
    fn is_extended_key_right_modifiers() {
        assert!(is_extended_key(VK_RCONTROL));
        assert!(is_extended_key(VK_RMENU));
        assert!(is_extended_key(VK_RWIN));
    }

    #[test]
    fn is_extended_key_regular_keys_are_not_extended() {
        assert!(!is_extended_key(VK_A));
        assert!(!is_extended_key(VK_SPACE));
        assert!(!is_extended_key(VK_RETURN));
        assert!(!is_extended_key(VK_LCONTROL));
        assert!(!is_extended_key(VK_LSHIFT));
        assert!(!is_extended_key(VK_F1));
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
        assert_ne!(ki.wScan, 0, "scan code should be populated");
    }

    #[test]
    #[expect(unsafe_code, reason = "accessing INPUT union field in test")]
    fn make_input_key_up_has_keyup_flag() {
        let input = make_input(VK_A, true);
        // SAFETY: we just created this as a keyboard input.
        let ki = unsafe { input.Anonymous.ki };
        assert_eq!(ki.dwFlags, KEYEVENTF_KEYUP);
        assert_ne!(ki.wScan, 0, "scan code should be populated");
    }

    #[test]
    #[expect(unsafe_code, reason = "accessing INPUT union field in test")]
    fn make_input_extended_key_down_has_extended_flag() {
        let input = make_input(VK_UP, false);
        // SAFETY: we just created this as a keyboard input.
        let ki = unsafe { input.Anonymous.ki };
        assert_eq!(ki.dwFlags, KEYEVENTF_EXTENDEDKEY);
    }

    #[test]
    #[expect(unsafe_code, reason = "accessing INPUT union field in test")]
    fn make_input_extended_key_up_has_both_flags() {
        let input = make_input(VK_UP, true);
        // SAFETY: we just created this as a keyboard input.
        let ki = unsafe { input.Anonymous.ki };
        assert_eq!(ki.dwFlags, KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY);
    }

    #[test]
    #[expect(
        unsafe_code,
        reason = "accessing INPUT union field and calling MapVirtualKeyW"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "test value: scan codes fit in u16"
    )]
    fn make_input_populates_scan_code() {
        let input = make_input(VK_A, false);
        // SAFETY: we just created this as a keyboard input.
        let ki = unsafe { input.Anonymous.ki };
        // Verify the scan code matches what MapVirtualKeyW returns for
        // this key on the current keyboard layout.
        // SAFETY: stateless Win32 lookup, no preconditions.
        let expected = unsafe { MapVirtualKeyW(u32::from(VK_A.0), MAPVK_VK_TO_VSC) } as u16;
        assert_eq!(ki.wScan, expected);
        assert_ne!(ki.wScan, 0, "scan code should be populated");
    }

    #[test]
    fn letter_to_vk_maps_via_arithmetic() {
        assert_eq!(letter_to_vk(b'A'), VK_A);
        assert_eq!(letter_to_vk(b'Z'), VK_Z);
        assert_eq!(letter_to_vk(b'M'), VK_M);
    }

    #[test]
    fn digit_to_vk_maps_via_arithmetic() {
        assert_eq!(digit_to_vk(b'0'), VK_0);
        assert_eq!(digit_to_vk(b'5'), VK_5);
        assert_eq!(digit_to_vk(b'9'), VK_9);
    }
}
