// Rust guideline compliant 2026-03-07

//! `InputForge` GUI — egui-based configuration interface.
//!
//! Provides a three-panel layout for managing physical input device
//! mappings to virtual vJoy outputs. The GUI communicates with the
//! engine thread through shared `AppState` and an `EngineCommand` channel.

pub mod app;
pub(crate) mod panels;
pub(crate) mod theme;
pub(crate) mod widgets;

use std::sync::Arc;
use std::sync::mpsc;

use parking_lot::RwLock;

use muda::MenuId;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use app::InputForgeApp;

/// Launch the `InputForge` GUI window.
///
/// Blocks the calling thread until the window is closed.
///
/// `start_minimized` is accepted for signature parity with
/// `inputforge_gui_dx::launch_gui` and is ignored here — `main.rs` already
/// gates the egui startup launch on `cli.start_minimized`. The parameter is
/// removed at F16 cleanup when the egui crate is deleted.
///
/// # Errors
///
/// Returns an error if the native window fails to initialize.
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
    start_minimized: bool,
) -> anyhow::Result<()> {
    let _ = start_minimized;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("InputForge")
            // 1280x800: 16:10 default fits most laptop and desktop displays.
            .with_inner_size([1280.0, 800.0])
            // 800x500: minimum to fit left panel (240) + center content.
            .with_min_inner_size([800.0, 500.0]),
        // Disable vsync to prevent the render thread from blocking on the
        // vertical blanking interval when a game is competing for the GPU.
        // With vsync on (the default), a missed vsync deadline stalls the
        // frame, which manifests as a black flash under heavy system load.
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "InputForge",
        options,
        Box::new(move |cc| {
            Ok(Box::new(InputForgeApp::new(
                cc,
                state,
                commands,
                tray_menu_ids,
                settings,
            )))
        }),
    )
    // eframe::Error is not Send + Sync (eframe 0.33 / glutin), so the blanket
    // anyhow::Error::from impl doesn't apply. Convert via Display; the source
    // chain is dropped, which is acceptable here — main.rs only uses %e.
    .map_err(|e| anyhow::Error::msg(e.to_string()))
}
