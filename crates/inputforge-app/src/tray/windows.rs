//! Wake hidden windows without depending on paint events.
use dioxus::desktop::tao::{platform::windows::WindowExtWindows, window::Window};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, WM_NULL},
};

#[allow(
    unsafe_code,
    reason = "posting a pointer-free wakeup to a live Tao window"
)]
pub(super) fn wake(window: &Window) -> bool {
    // SAFETY: the borrowed window owns this HWND and WM_NULL carries no pointers.
    unsafe {
        PostMessageW(
            Some(HWND(window.hwnd() as *mut std::ffi::c_void)),
            WM_NULL,
            WPARAM(0),
            LPARAM(0),
        )
    }
    .is_ok()
}
