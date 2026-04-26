//! Dioxus Desktop GUI for `InputForge`.

mod app;
mod bridge;
mod context;
mod tray;

pub mod components;
pub mod icons;
pub mod theme;

use std::sync::{Arc, mpsc};

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use muda::MenuId;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::RawHandles;

/// Launch the Dioxus Desktop GUI. Blocks the calling thread on the OS event
/// loop (wry/tao underneath) — matches the egui crate's `eframe::run_native`
/// blocking semantics.
///
/// `tray_menu_ids` is accepted for signature parity with `inputforge_gui::launch_gui`
/// but is stubbed at F1; F3 wires the listener task that consumes it.
///
/// # Errors
///
/// Currently always returns `Ok(())`. The `Result` return type exists for
/// signature parity with `inputforge_gui::launch_gui`, enabling `cfg`-gated
/// dispatch in `inputforge-app::main`. Future tasks may surface engine or
/// runtime initialization failures via this `Result`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "signature parity with inputforge_gui::launch_gui — main.rs dispatches \
              both crates via a cfg-gated `use` line; changing to `&` here would \
              break the call site when the egui crate is swapped in"
)]
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
) -> anyhow::Result<()> {
    tracing::debug!(?tray_menu_ids, "tray wiring stubbed until F3");

    let handles = RawHandles {
        state,
        commands,
        settings: Arc::new(settings),
    };

    let window = WindowBuilder::new()
        .with_title("InputForge")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0));

    let cfg = Config::new().with_window(window);

    LaunchBuilder::desktop()
        .with_cfg(cfg)
        .with_context(handles)
        .launch(app::app_root);

    Ok(())
}
