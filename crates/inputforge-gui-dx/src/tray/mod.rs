//! Tray bridge — observes `dioxus-desktop`'s forwarded muda events via the
//! `use_muda_event_handler` hook, routes through a bounded
//! `tokio::sync::mpsc`, and dispatches in a Dioxus task.
//!
//! ## Deviation from spec
//!
//! The design spec (`2026-04-26-f3-app-shell-tray-bridge-design.md`) calls
//! for `Config::with_custom_event_handler` matching on
//! `UserWindowEvent::MudaMenuEvent`. That type lives in
//! `dioxus_desktop::ipc::UserWindowEvent` — but `mod ipc;` is private in
//! `dioxus-desktop` 0.7.6, so the type is unreachable from external crates
//! (verified at `dioxus-desktop-0.7.6/src/lib.rs:21`). `dioxus-desktop`
//! exposes `use_muda_event_handler` (`hooks.rs:44`, re-exported via
//! `pub use hooks::*;`) which delivers exactly the same `muda::MenuEvent`
//! payload through a hook callback running in the event-loop context.
//!
//! Architectural invariants are preserved: observe-only handler →
//! `try_send` into bounded `tokio::sync::mpsc` → drained by a Dioxus task
//! that calls `lifecycle::*` and engine commands. Pure routing remains in
//! `tray/action.rs`. The handle returned by the hook is auto-cleaned on
//! scope teardown.

pub(crate) mod action;

use dioxus::desktop::use_muda_event_handler;
use dioxus::prelude::*;
use tokio::sync::mpsc;

use inputforge_core::engine::EngineCommand;
use inputforge_core::state::EngineStatus;

use crate::context::AppContext;
use crate::lifecycle;

use self::action::{TrayAction, TrayMenuIds};

/// Capacity for the tray-action channel.
///
/// The channel buffers between the muda event handler (runs synchronously on
/// the tao event-loop thread, so must never block) and the listener task
/// (runs in the Dioxus runtime). The worst case is a burst of clicks during
/// a stalled listener task — drainage normally resumes within single-digit
/// ms, and tray clicks are human-paced (≤ ~5 Hz peak from frantic clicking).
/// Cap=2 would technically suffice; 8 gives generous headroom. Overflow is
/// logged via `tracing::warn!` and the action is dropped (see
/// `install_event_handler`), which is preferable to blocking the event loop.
pub(crate) const CHANNEL_CAPACITY: usize = 8;

/// Install the muda event handler. Must be called from inside a Dioxus
/// scope (it wraps `use_muda_event_handler`, which is itself a hook).
///
/// The handler runs synchronously on the tao event-loop thread; it must
/// not block. We `try_send` and log any overflow rather than wait —
/// overflow is effectively impossible at human input rates, but a dropped
/// send must never deadlock the event loop. The handler is observe-only;
/// routing happens in `TrayAction::from_event` (pure) and dispatch in the
/// listener task spawned via `spawn_listener_task`.
pub(crate) fn install_event_handler(ids: TrayMenuIds, tx: mpsc::Sender<TrayAction>) {
    use_muda_event_handler(move |menu_ev| {
        if let Some(action) = TrayAction::from_event(menu_ev, &ids) {
            if let Err(err) = tx.try_send(action) {
                tracing::warn!(?err, "tray channel overflow; dropping action");
            }
        }
    });
}

/// Spawn the listener task. Called from `app_root`'s `use_hook` so the task
/// is tied to the Dioxus runtime lifetime and auto-cancelled on teardown.
pub(crate) fn spawn_listener_task(mut rx: mpsc::Receiver<TrayAction>, ctx: AppContext) {
    spawn(async move {
        while let Some(action) = rx.recv().await {
            match action {
                TrayAction::Show => lifecycle::show_window(),
                TrayAction::Toggle => dispatch_toggle(&ctx),
                TrayAction::Quit => lifecycle::request_quit(),
            }
        }
    });
}

/// Translate a Toggle action into the appropriate `EngineCommand` based on
/// current engine status, then send it on the engine command channel.
///
/// `AppContext.commands` is `std::sync::mpsc::Sender<EngineCommand>` (an
/// unbounded std channel from F1) — its `send` is non-blocking for unbounded
/// channels, returning `Err` only if the receiver has been dropped. We
/// discard the error: at that point the engine is already gone and the user
/// is about to learn so via the normal shutdown path.
fn dispatch_toggle(ctx: &AppContext) {
    let status = ctx.state.read().engine_status;
    let cmd = match status {
        EngineStatus::Running => EngineCommand::Deactivate,
        EngineStatus::Paused | EngineStatus::Stopped => EngineCommand::Activate,
    };
    let _ = ctx.commands.send(cmd);
}
