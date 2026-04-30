//! Window-lifecycle helpers. Three functions, all called from inside a
//! Dioxus scope (so `dioxus::desktop::window()` resolves correctly).
//!
//! No close-hook gate: Dioxus owns close-requested handling. X-click hide
//! is set up at launch via `WindowCloseBehaviour::WindowHides` in
//! `lib.rs::launch_gui`; tray Quit flips the per-window close behavior to
//! `WindowCloses` then triggers close, the event loop exits because
//! `exit_on_last_window_close` is true (the default; F3 does not override).
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
///
/// `quit_requested` in `AppState` is **not** read on the Dioxus path
/// (egui still uses it). The close-behavior switch is the entire Quit
/// pathway, there is no flag to gate, no close-hook to wire.
pub(crate) fn request_quit() {
    let w = window();
    w.set_close_behavior(WindowCloseBehaviour::WindowCloses);
    w.close();
}

/// Apply --start-minimized once during `app_root` mount.
///
/// **Egui/Dioxus divergence.** The egui path skips `launch_gui_blocking`
/// entirely when `--start-minimized` is set, no window is created until
/// tray Show requests it. The Dioxus path always creates the `WebView2`
/// window (and the polling + listener tasks) and hides it via
/// `set_visible(false)`. Dioxus 0.7's `tao::EventLoop::run` is one-shot,
/// so a tray-triggered relaunch isn't viable without restructuring the
/// whole event loop. UX is identical (no window visible at startup, tray
/// Show opens it) but resource usage is not, `--features gui-dioxus`
/// pays the `WebView2` + task cost up-front regardless of `--start-minimized`.
pub(crate) fn apply_start_minimized(start_minimized: bool) {
    if start_minimized {
        window().set_visible(false);
    }
}
