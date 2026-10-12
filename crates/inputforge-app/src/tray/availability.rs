//! Shared window policy: only hide when the tray can restore the editor.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::window;
use dioxus::desktop::{WindowCloseBehaviour, window};
use inputforge_core::state::AppState;
use parking_lot::RwLock;

pub(crate) struct WindowPolicy {
    pub visible: bool,
    pub hide_on_close: bool,
}

impl WindowPolicy {
    pub(crate) fn new(available: bool, minimized: bool) -> Self {
        Self {
            visible: !available || !minimized,
            hide_on_close: available,
        }
    }

    pub(crate) fn close_behavior(&self) -> WindowCloseBehaviour {
        if self.hide_on_close {
            WindowCloseBehaviour::WindowHides
        } else {
            WindowCloseBehaviour::WindowCloses
        }
    }
}

fn should_restore(previous: bool, available: bool) -> bool {
    previous && !available
}

const UNAVAILABLE_NOTICE: &str =
    "Tray access is unavailable; the editor stays visible and closing it quits InputForge.";

pub(crate) fn monitor(
    available: Arc<AtomicBool>,
    mut previous: bool,
    state: Arc<RwLock<AppState>>,
) -> impl FnMut() + Clone {
    let w = window();
    if !previous {
        state.write().session.notice = Some(UNAVAILABLE_NOTICE.into());
    }
    move || {
        // Start from the behavior actually configured on the window, so a
        // host loss between construction and mounting still restores it.
        let current = available.load(Ordering::Acquire);
        if current != previous {
            w.set_close_behavior(WindowPolicy::new(current, false).close_behavior());
            if should_restore(previous, current) {
                window::show_window();
                state.write().session.notice = Some(UNAVAILABLE_NOTICE.into());
            }
            if current {
                let mut state = state.write();
                if state.session.notice.as_deref() == Some(UNAVAILABLE_NOTICE) {
                    state.session.notice = None;
                }
            }
            previous = current;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimized_start_requires_a_usable_tray() {
        assert!(!WindowPolicy::new(true, true).visible);
        assert!(WindowPolicy::new(false, true).visible);
        assert!(WindowPolicy::new(true, false).visible);
        assert!(WindowPolicy::new(false, false).visible);
    }

    #[test]
    fn close_to_tray_requires_a_usable_tray() {
        assert!(WindowPolicy::new(true, false).hide_on_close);
        assert!(!WindowPolicy::new(false, false).hide_on_close);
    }

    #[test]
    fn host_loss_restores_but_host_arrival_does_not_hide() {
        assert!(should_restore(true, false));
        assert!(!should_restore(false, true));
        assert!(!should_restore(false, false));
        assert!(!should_restore(true, true));
    }
}
