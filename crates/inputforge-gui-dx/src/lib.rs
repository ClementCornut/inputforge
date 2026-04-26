//! Dioxus Desktop GUI for `InputForge`.

mod app;
mod bridge;
mod context;
mod lifecycle;
mod shell;
pub mod toast;
mod tray;

pub mod components;
pub mod icons;
pub mod theme;

pub use toast::{ToastLevel, ToastQueue};

use std::sync::{Arc, mpsc};

use dioxus::desktop::{Config, LogicalSize, WindowBuilder, WindowCloseBehaviour};
use dioxus::prelude::*;
use muda::MenuId;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::RawHandles;
use crate::tray::action::TrayMenuIds;

/// Per-launch parameters carried from `launch_gui` into `app_root` via
/// `LaunchBuilder::with_context`.
///
/// `tray_menu_ids` flows through here (rather than through a separate context
/// type) because it's only consumed by `app_root` once, during initial mount,
/// to install the muda event handler. The tokio mpsc channel that the handler
/// pushes onto is created INSIDE `app_root` (not here) — the original spec
/// design carried `Arc<Mutex<Option<Receiver<TrayAction>>>>` through here, but
/// the deviation in Tasks 10/11 (using `use_muda_event_handler` instead of
/// `Config::with_custom_event_handler`) means both channel halves can live
/// inside `app_root`'s `use_hook` body.
#[derive(Clone)]
pub(crate) struct LaunchParams {
    pub start_minimized: bool,
    pub tray_menu_ids: TrayMenuIds,
}

/// Launch the Dioxus Desktop GUI. Blocks the calling thread on the OS event
/// loop (wry/tao underneath) — matches the egui crate's `eframe::run_native`
/// blocking semantics.
///
/// `tray_menu_ids` flow through `LaunchParams::tray_menu_ids` into
/// `app_root`, which calls `tray::install_event_handler(...)` from inside a
/// `use_hook`. The handler observes muda menu events forwarded by Dioxus,
/// routes them via `TrayAction::from_event`, and pushes onto a bounded
/// `tokio::sync::mpsc` channel created inside `app_root`. A Dioxus task
/// (also spawned from `app_root`) drains the channel and dispatches.
///
/// # Errors
///
/// Currently always returns `Ok(())`. The `Result` return type exists for
/// signature parity with `inputforge_gui::launch_gui`, enabling `cfg`-gated
/// dispatch in `inputforge-app::main`. Future tasks may surface engine or
/// runtime initialization failures via this `Result`.
#[allow(
    clippy::needless_pass_by_value,
    reason = "signature parity with inputforge_gui::launch_gui — main.rs dispatches \
              both crates via a cfg-gated `use` line; changing to `&` here would \
              break the call site when the egui crate is swapped in. `#[allow]` \
              rather than `#[expect]` because the current body moves every arg, \
              so the lint may not fire today; future body changes that borrow \
              instead of move must remain suppressed."
)]
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
    start_minimized: bool,
) -> anyhow::Result<()> {
    let (show, toggle, quit) = tray_menu_ids;
    let menu_ids = TrayMenuIds { show, toggle, quit };

    let handles = RawHandles {
        state,
        commands,
        settings: Arc::new(settings),
    };
    let params = LaunchParams {
        start_minimized,
        tray_menu_ids: menu_ids,
    };

    let window = WindowBuilder::new()
        .with_title("InputForge")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0));

    let cfg = Config::new()
        .with_window(window)
        .with_close_behaviour(WindowCloseBehaviour::WindowHides);
    // exit_on_last_window_close left at its default (true).
    // Custom event handler NOT installed here — Task 10/11 deviation: the
    // handler is installed via `tray::install_event_handler` (a hook
    // wrapping `use_muda_event_handler`) from inside `app_root`'s `use_hook`,
    // because `dioxus_desktop::ipc::UserWindowEvent` is private in 0.7.6.

    LaunchBuilder::desktop()
        .with_cfg(cfg)
        .with_context(handles)
        .with_context(params)
        .launch(app::app_root);

    Ok(())
}
