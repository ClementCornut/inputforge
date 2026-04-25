//! Dioxus Desktop GUI for `InputForge`.

mod app;
mod bridge;
mod context;

use std::sync::Arc;
use std::sync::mpsc;

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
/// Currently does not fail — `LaunchBuilder::launch` is non-returning. The
/// `Result` return type matches the egui crate's signature for `cfg`-gated
/// dispatch in `inputforge-app::main`. Future tasks may surface engine /
/// runtime errors here.
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
    let _ = tray_menu_ids;

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
