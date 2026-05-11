// Rust guideline compliant 2026-05-11

use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN as MOUSE_LEFTDOWN,
    MOUSEEVENTF_LEFTUP as MOUSE_LEFTUP, MOUSEEVENTF_MIDDLEDOWN as MOUSE_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP as MOUSE_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN as MOUSE_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP as MOUSE_RIGHTUP, MOUSEEVENTF_WHEEL as MOUSE_WHEEL,
    MOUSEEVENTF_XDOWN as MOUSE_XDOWN, MOUSEEVENTF_XUP as MOUSE_XUP, MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{
    WHEEL_DELTA as WIN32_WHEEL_DELTA, XBUTTON1 as WIN32_XBUTTON1, XBUTTON2 as WIN32_XBUTTON2,
};

use crate::action::MouseTarget;
use crate::error::{EngineError, Result};

use super::traits::MouseSink;

/// Mouse output that simulates button and wheel input via Win32 `SendInput`.
#[derive(Debug, Default)]
pub struct MouseOutput;

impl MouseOutput {
    /// Create a new `MouseOutput`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl MouseSink for MouseOutput {
    fn button_down(&mut self, target: MouseTarget) -> Result<()> {
        let (flags, data) =
            button_flags(target, true).ok_or_else(|| EngineError::InvalidConfig {
                reason: format!("mouse wheel target cannot be pressed as a button: {target:?}"),
            })?;

        send_mouse_input(make_mouse_input(flags, mouse_data_to_i32(data)))
    }

    fn button_up(&mut self, target: MouseTarget) -> Result<()> {
        let (flags, data) =
            button_flags(target, false).ok_or_else(|| EngineError::InvalidConfig {
                reason: format!("mouse wheel target cannot be released as a button: {target:?}"),
            })?;

        send_mouse_input(make_mouse_input(flags, mouse_data_to_i32(data)))
    }

    fn wheel(&mut self, target: MouseTarget) -> Result<()> {
        let data = wheel_data(target).ok_or_else(|| EngineError::InvalidConfig {
            reason: format!("mouse button target cannot be scrolled as a wheel: {target:?}"),
        })?;

        send_mouse_input(make_mouse_input(MOUSE_WHEEL, data))
    }
}

fn button_flags(target: MouseTarget, down: bool) -> Option<(MOUSE_EVENT_FLAGS, u32)> {
    match (target, down) {
        (MouseTarget::LeftButton, true) => Some((MOUSE_LEFTDOWN, 0)),
        (MouseTarget::LeftButton, false) => Some((MOUSE_LEFTUP, 0)),
        (MouseTarget::RightButton, true) => Some((MOUSE_RIGHTDOWN, 0)),
        (MouseTarget::RightButton, false) => Some((MOUSE_RIGHTUP, 0)),
        (MouseTarget::MiddleButton, true) => Some((MOUSE_MIDDLEDOWN, 0)),
        (MouseTarget::MiddleButton, false) => Some((MOUSE_MIDDLEUP, 0)),
        (MouseTarget::BackButton, true) => Some((MOUSE_XDOWN, u32::from(WIN32_XBUTTON1))),
        (MouseTarget::BackButton, false) => Some((MOUSE_XUP, u32::from(WIN32_XBUTTON1))),
        (MouseTarget::ForwardButton, true) => Some((MOUSE_XDOWN, u32::from(WIN32_XBUTTON2))),
        (MouseTarget::ForwardButton, false) => Some((MOUSE_XUP, u32::from(WIN32_XBUTTON2))),
        (MouseTarget::WheelUp | MouseTarget::WheelDown, _) => None,
    }
}

fn wheel_data(target: MouseTarget) -> Option<i32> {
    match target {
        MouseTarget::WheelUp => Some(WIN32_WHEEL_DELTA as i32),
        MouseTarget::WheelDown => Some(-(WIN32_WHEEL_DELTA as i32)),
        MouseTarget::LeftButton
        | MouseTarget::RightButton
        | MouseTarget::MiddleButton
        | MouseTarget::BackButton
        | MouseTarget::ForwardButton => None,
    }
}

fn make_mouse_input(flags: MOUSE_EVENT_FLAGS, mouse_data: impl Into<i32>) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: mouse_data.into() as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[expect(
    clippy::cast_possible_wrap,
    reason = "mouse button data values are small"
)]
fn mouse_data_to_i32(data: u32) -> i32 {
    data as i32
}

/// Call Win32 `SendInput` with a single mouse input.
///
/// # Errors
///
/// Returns [`EngineError::OutputFailed`] if `SendInput` does not inject the
/// requested mouse event.
#[expect(unsafe_code, reason = "SendInput and GetLastError are Win32 FFI calls")]
fn send_mouse_input(input: INPUT) -> Result<()> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "size_of::<INPUT>() is 40 bytes, fits in i32"
    )]
    let cb_size = size_of::<INPUT>() as i32;

    let inputs = [input];

    // SAFETY: `inputs` is a valid one-element slice of `INPUT` structs, and
    // `cb_size` matches the size of each element. `SendInput` reads from the
    // slice without retaining references.
    let sent = unsafe { SendInput(&inputs, cb_size) };

    if sent == 1 {
        Ok(())
    } else {
        // SAFETY: `GetLastError` is a stateless Win32 call with no preconditions.
        let last_err = unsafe { GetLastError() };
        Err(EngineError::OutputFailed {
            reason: format!("SendInput sent {sent}/1 mouse events (last error: {last_err:?})"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
        MOUSEEVENTF_XUP,
    };
    use windows::Win32::UI::WindowsAndMessaging::{WHEEL_DELTA, XBUTTON1};

    #[test]
    fn left_button_maps_to_down_and_up_flags() {
        assert_eq!(
            button_flags(MouseTarget::LeftButton, true).unwrap().0,
            MOUSEEVENTF_LEFTDOWN
        );
        assert_eq!(
            button_flags(MouseTarget::LeftButton, false).unwrap().0,
            MOUSEEVENTF_LEFTUP
        );
    }

    #[test]
    fn back_button_sets_xbutton_data() {
        let down = button_flags(MouseTarget::BackButton, true).unwrap();
        let up = button_flags(MouseTarget::BackButton, false).unwrap();

        assert_eq!(down.0, MOUSEEVENTF_XDOWN);
        assert_eq!(down.1, u32::from(XBUTTON1));
        assert_eq!(up.0, MOUSEEVENTF_XUP);
        assert_eq!(up.1, u32::from(XBUTTON1));
    }

    #[test]
    fn wheel_targets_map_to_standard_notches() {
        assert_eq!(
            wheel_data(MouseTarget::WheelUp).unwrap(),
            WHEEL_DELTA as i32
        );
        assert_eq!(
            wheel_data(MouseTarget::WheelDown).unwrap(),
            -(WHEEL_DELTA as i32)
        );
        assert!(wheel_data(MouseTarget::LeftButton).is_none());
    }

    #[test]
    #[expect(unsafe_code, reason = "accessing INPUT union field in test")]
    fn wheel_input_uses_mouse_wheel_flag() {
        let input = make_mouse_input(MOUSEEVENTF_WHEEL, WHEEL_DELTA as i32);

        assert_eq!(input.r#type, INPUT_MOUSE);
        assert_eq!(unsafe { input.Anonymous.mi.dwFlags }, MOUSEEVENTF_WHEEL);
        assert_eq!(unsafe { input.Anonymous.mi.mouseData }, WHEEL_DELTA);
    }
}
