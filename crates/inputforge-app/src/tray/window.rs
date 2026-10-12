//! Desktop window actions owned by the application.

use dioxus::desktop::{WindowCloseBehaviour, window};

/// Tray Show, bring the window back to foreground.
pub(crate) fn show_window() {
    let w = window();
    w.set_minimized(false);
    w.set_visible(true);
    w.set_focus();
}

/// Tray Quit, switch this window's close behavior to `WindowCloses`,
/// then trigger close. Dioxus destroys the window, observes zero remaining
/// webviews, and the event loop exits because `exit_on_last_window_close`
/// is true. The configured `LoopDestroyed` handler shuts down and joins the
/// engine before Tao exits the process.
pub(crate) fn request_quit() {
    let w = window();
    w.set_close_behavior(WindowCloseBehaviour::WindowCloses);
    w.close();
}
