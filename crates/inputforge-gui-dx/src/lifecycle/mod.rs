//! Window-lifecycle helpers. Two functions, all called from inside a
//! Dioxus scope (so `dioxus::desktop::window()` resolves correctly).
//!
//! No close-hook gate: Dioxus owns close-requested handling. X-click hide
//! is set up at launch via `WindowCloseBehaviour::WindowHides` in
//! `lib.rs::launch_gui`; tray Quit flips the per-window close behavior to
//! `WindowCloses` then triggers close, the event loop exits because
//! `exit_on_last_window_close` is true (the default; F3 does not override).
//!
//! Start-minimized is applied at construction via
//! `WindowBuilder::with_visible(false)` in `lib.rs::launch_gui`, not from
//! a hook here, hiding post-mount with `set_visible(false)` races the
//! `WebView2` first paint and leaves the window visible under `dx run`.
//!
//! ## API spelling note
//!
//! `dioxus-desktop` 0.7.6 is asymmetric:
//! - `DesktopService::set_close_behavior`, US spelling (method)
//!   (`dioxus-desktop-0.7.6/src/desktop_context.rs:177`)
//! - `WindowCloseBehaviour`, UK spelling (enum)
//!   (`dioxus-desktop-0.7.6/src/config.rs:26`)
//! - `Config::with_close_behaviour`, UK spelling (Task 13 will use this)
//!   (`dioxus-desktop-0.7.6/src/config.rs:204`)

use dioxus::desktop::{WindowCloseBehaviour, window};

/// Tray Show, bring the window back to foreground.
pub(crate) fn show_window() {
    let w = window();
    w.set_visible(true);
    w.set_focus();
}

/// Tray Quit, switch this window's close behavior to `WindowCloses`,
/// then trigger close. Dioxus destroys the window, observes zero remaining
/// webviews, and the event loop exits because `exit_on_last_window_close`
/// is true (the default, F3 does not override). `launch_gui` returns;
/// `main.rs::shutdown()` then runs.
pub(crate) fn request_quit() {
    let w = window();
    w.set_close_behavior(WindowCloseBehaviour::WindowCloses);
    w.close();
}
