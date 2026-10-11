//! Tray bridge, forwards the tray's muda events through a bounded
//! `tokio::sync::mpsc` and dispatches in a Dioxus task.
//!
//! The tray uses the workspace's muda version, while `dioxus-desktop`
//! uses its own older version with a separate global event handler.
//! Register directly with the tray's version so clicks reach this channel.
//! The callback only routes and `try_send`s; the Dioxus task performs GUI
//! and engine actions. Muda retains the callback for the process lifetime;
//! the application launches one tray-enabled root per process.

pub(crate) mod action;

use std::cell::RefCell;

use dioxus::prelude::*;
use muda::{MenuEvent, MenuItem};
use tokio::sync::mpsc;

use inputforge_core::engine::EngineCommand;
use inputforge_core::state::EngineStatus;

use crate::lifecycle;
use crate::{
    context::AppContext,
    frame::{PanelSlot, ViewState},
};

use self::action::{TrayAction, TrayMenuIds};

thread_local! {
    /// Hand-off slot for the tray's toggle `MenuItem`.
    ///
    /// `MenuItem` wraps `Rc<RefCell<...>>` so it is neither `Send` nor
    /// `Sync` and cannot ride through `LaunchBuilder::with_context`
    /// (which bounds its payload by `Send + Sync`). Both `launch_gui`
    /// and the Dioxus desktop runtime run on the same main thread, so
    /// we use a thread-local install / take handoff: the app-side caller
    /// installs the cloned handle, and `app_root`'s first mount takes
    /// it back out via `take_toggle_menu_item`.
    static TOGGLE_MENU_ITEM: RefCell<Option<MenuItem>> = const { RefCell::new(None) };
}

/// Install the tray's toggle `MenuItem` for later pickup by `app_root`.
///
/// Must be called on the same thread that will host the Dioxus runtime
/// (the application's main thread). Overwrites any prior installed value.
pub(crate) fn install_toggle_menu_item(item: MenuItem) {
    TOGGLE_MENU_ITEM.with(|cell| *cell.borrow_mut() = Some(item));
}

/// Take the previously installed toggle `MenuItem`.
///
/// Returns `None` if no value was installed (e.g., test harnesses that
/// do not exercise the tray label sync effect).
pub(crate) fn take_toggle_menu_item() -> Option<MenuItem> {
    TOGGLE_MENU_ITEM.with(|cell| cell.borrow_mut().take())
}

/// Capacity for the tray-action channel.
///
/// The channel buffers between the muda event handler (runs synchronously on
/// the tao event-loop thread, so must never block) and the listener task
/// (runs in the Dioxus runtime). The worst case is a burst of clicks during
/// a stalled listener task, drainage normally resumes within single-digit
/// ms, and tray clicks are human-paced (≤ ~5 Hz peak from frantic clicking).
/// Cap=2 would technically suffice; 8 gives generous headroom. Overflow is
/// logged via `tracing::warn!` and the action is dropped (see
/// `install_event_handler`), which is preferable to blocking the event loop.
pub(crate) const CHANNEL_CAPACITY: usize = 8;

/// Install the tray's muda event handler once from inside a Dioxus scope.
///
/// The handler runs synchronously on the tao event-loop thread; it must
/// not block. We `try_send` and log any overflow rather than wait -
/// overflow is effectively impossible at human input rates, but a dropped
/// send must never deadlock the event loop. The handler is observe-only;
/// routing happens in `TrayAction::from_event` (pure) and dispatch in the
/// listener task spawned via `spawn_listener_task`.
pub(crate) fn install_event_handler(ids: TrayMenuIds, tx: mpsc::Sender<TrayAction>) {
    use_hook(move || {
        MenuEvent::set_event_handler(Some(move |menu_ev| {
            forward_menu_event(&menu_ev, &ids, &tx);
        }));
    });
}

/// Route a tray event without blocking the native event-loop thread.
fn forward_menu_event(ev: &MenuEvent, ids: &TrayMenuIds, tx: &mpsc::Sender<TrayAction>) {
    if let Some(action) = TrayAction::from_event(ev, ids)
        && let Err(err) = tx.try_send(action)
    {
        tracing::warn!(?err, "tray channel overflow; dropping action");
    }
}

/// Spawn the listener task. Called from `app_root`'s `use_hook` so the task
/// is tied to the Dioxus runtime lifetime and auto-cancelled on teardown.
pub(crate) fn spawn_listener_task(
    mut rx: mpsc::Receiver<TrayAction>,
    ctx: AppContext,
    view: ViewState,
) {
    spawn(async move {
        while let Some(action) = rx.recv().await {
            match action {
                TrayAction::Show => lifecycle::show_window(),
                TrayAction::Toggle => dispatch_toggle(&ctx, view),
                TrayAction::Quit => lifecycle::request_quit(),
            }
        }
    });
}

/// Translate a Toggle action into the appropriate `EngineCommand` based on
/// current engine status, then send it on the engine command channel.
///
/// `AppContext.commands` is `std::sync::mpsc::Sender<EngineCommand>` (an
/// unbounded std channel from F1), its `send` is non-blocking for unbounded
/// channels, returning `Err` only if the receiver has been dropped. We
/// discard the error: at that point the engine is already gone and the user
/// is about to learn so via the normal shutdown path.
fn dispatch_toggle(ctx: &AppContext, mut view: ViewState) {
    if ctx.controller_changes_pending() {
        lifecycle::show_window();
        view.via_calibration.set(false);
        view.panel_slot.set(PanelSlot::Devices);
        return;
    }
    let status = ctx.state.read().engine_status;
    let cmd = match status {
        EngineStatus::Running | EngineStatus::Starting => EngineCommand::Deactivate,
        EngineStatus::Stopped | EngineStatus::Faulted => EngineCommand::Activate,
    };
    let _ = ctx.commands.send(cmd);
}

/// Label for the tray's toggle menu item, derived from current engine status.
///
/// Mirrors the routing action dispatched by [`dispatch_toggle`].
pub(crate) fn toggle_label(status: EngineStatus, pending: bool) -> &'static str {
    if pending {
        return "Review controller changes";
    }
    match status {
        EngineStatus::Running | EngineStatus::Starting => "Stop",
        EngineStatus::Stopped | EngineStatus::Faulted => "Start",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_event_sends_matching_action() {
        let ids = TrayMenuIds {
            show: muda::MenuId::new("show"),
            toggle: muda::MenuId::new("toggle"),
            quit: muda::MenuId::new("quit"),
        };
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        forward_menu_event(
            &MenuEvent {
                id: ids.toggle.clone(),
            },
            &ids,
            &tx,
        );
        assert_eq!(rx.try_recv().unwrap(), TrayAction::Toggle);
        forward_menu_event(
            &MenuEvent {
                id: muda::MenuId::new("unknown"),
            },
            &ids,
            &tx,
        );
        assert_eq!(rx.try_recv(), Err(mpsc::error::TryRecvError::Empty));
    }

    #[test]
    fn menu_event_drops_overflow_without_blocking() {
        let ids = TrayMenuIds {
            show: muda::MenuId::new("show"),
            toggle: muda::MenuId::new("toggle"),
            quit: muda::MenuId::new("quit"),
        };
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(TrayAction::Show).unwrap();
        forward_menu_event(
            &MenuEvent {
                id: ids.quit.clone(),
            },
            &ids,
            &tx,
        );
        assert_eq!(rx.try_recv().unwrap(), TrayAction::Show);
        assert_eq!(rx.try_recv(), Err(mpsc::error::TryRecvError::Empty));
    }

    #[test]
    fn toggle_label_matches_dispatch_mapping() {
        assert_eq!(toggle_label(EngineStatus::Running, false), "Stop");
        assert_eq!(toggle_label(EngineStatus::Stopped, false), "Start");
    }
}
