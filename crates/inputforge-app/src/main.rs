// Rust guideline compliant 2026-03-07

//! `InputForge`, desktop application entry point.
//!
//! Wires together the engine thread, system tray icon, and the Dioxus GUI
//! window. The engine runs on a dedicated thread (`SDL3` is `!Send`).

mod cli;
mod tray;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use clap::Parser;
use mimalloc::MiMalloc;
use parking_lot::RwLock;

use inputforge_core::device::{DeviceHider, NoOpDeviceHider, Sdl3Input};
use inputforge_core::engine::{Engine, EngineCommand};
use inputforge_core::output::{KeyboardOutput, VJoyOutput};
use inputforge_core::profile::Profile;
use inputforge_core::profile::manager::ensure_default_profile;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::cli::Cli;
use crate::tray::AppTray;

use inputforge_gui_dx::launch_gui;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    tracing::info!(?cli, "starting InputForge");

    // Shared state and command channel.
    let state = Arc::new(RwLock::new(AppState::new()));
    let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>();

    // Load settings and determine which profile to use.
    let mut settings = AppSettings::load();

    let profile_path = if let Some(ref path) = cli.profile {
        // CLI argument takes priority.
        let profile = Profile::load(path)?;
        let mut guard = state.write();
        *guard = AppState::with_profile(profile);
        guard.profile_path = Some(path.clone());
        path.clone()
    } else {
        // Try last-used profile from settings, fall back to default.
        match settings.last_profile {
            Some(ref last) if last.exists() => match Profile::load(last) {
                Ok(profile) => {
                    let mut guard = state.write();
                    *guard = AppState::with_profile(profile);
                    guard.profile_path = Some(last.clone());
                    last.clone()
                }
                Err(e) => {
                    tracing::warn!(
                        path = %last.display(),
                        %e,
                        "failed to load last-used profile, falling back to default"
                    );
                    let default_path = ensure_default_profile()?;
                    let profile = Profile::load(&default_path)?;
                    let mut guard = state.write();
                    *guard = AppState::with_profile(profile);
                    guard.profile_path = Some(default_path.clone());
                    default_path
                }
            },
            _ => {
                let default_path = ensure_default_profile()?;
                let profile = Profile::load(&default_path)?;
                let mut guard = state.write();
                *guard = AppState::with_profile(profile);
                guard.profile_path = Some(default_path.clone());
                default_path
            }
        }
    };

    // Persist the loaded profile path as last-used.
    settings.last_profile = Some(profile_path);
    if let Err(e) = settings.save() {
        tracing::warn!(%e, "failed to save application settings");
    }

    // Spawn the engine on a dedicated thread. All !Send types (SDL3)
    // are created on this thread.
    let engine_state = Arc::clone(&state);
    let engine_handle = thread::Builder::new()
        .name("engine".into())
        .spawn(move || run_engine(engine_state, cmd_rx))?;

    // Send activate command if requested.
    if cli.enable {
        cmd_tx.send(EngineCommand::Activate)?;
    }

    // Create the system tray icon (always visible).
    let tray = AppTray::new(Arc::clone(&state))?;

    if let Err(e) = launch_gui(
        Arc::clone(&state),
        cmd_tx.clone(),
        tray.menu_item_ids(),
        tray.toggle_menu_item(),
        cli.start_minimized,
    ) {
        tracing::error!(%e, "GUI exited with error");
    }
    // launch_gui only returns on real Quit (tray Quit click). Fall
    // through to shutdown, no run_tray_loop, no drain_stale_gui_events.
    // The window-hides-on-X behavior is owned by Dioxus via
    // WindowCloseBehaviour::WindowHides set in launch_gui.

    // Graceful shutdown.
    shutdown(cmd_tx, engine_handle);

    Ok(())
}

// ---------------------------------------------------------------------------
// Engine thread
// ---------------------------------------------------------------------------

/// Run the engine on a dedicated thread.
///
/// Creates all `!Send` I/O types here (`SDL3`) and enters the
/// engine loop. Wrapped in [`std::panic::catch_unwind`] so that `Drop`
/// impls (`vJoy` flush) still run on panic.
fn run_engine(state: Arc<RwLock<AppState>>, commands: mpsc::Receiver<EngineCommand>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_engine_inner(state, commands)
    }));

    match result {
        Ok(Ok(())) => tracing::info!("engine thread exited cleanly"),
        Ok(Err(e)) => tracing::error!(%e, "engine thread exited with error"),
        Err(panic_payload) => {
            let msg = panic_payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic_payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("<non-string panic>");
            tracing::error!(panic = msg, "engine thread panicked");
        }
    }
}

/// Inner engine setup and run loop, separated so `catch_unwind` covers
/// both construction and execution.
fn run_engine_inner(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Receiver<EngineCommand>,
) -> Result<(), inputforge_core::error::EngineError> {
    let input = Box::new(Sdl3Input::new()?);
    let output = Box::new(VJoyOutput::new()?);
    let keyboard = Box::new(KeyboardOutput::new());
    let hider: Box<dyn DeviceHider> = Box::new(NoOpDeviceHider);

    let mut engine = Engine::new(
        input,
        output,
        keyboard,
        hider,
        state,
        commands,
        AppSettings::load(),
        AppSettings::settings_path(),
    );
    engine.run()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Shutdown
// ---------------------------------------------------------------------------

/// Send shutdown to the engine and wait for the thread to exit.
fn shutdown(cmd_tx: mpsc::Sender<EngineCommand>, engine_handle: thread::JoinHandle<()>) {
    // Send shutdown command. If engine is already dead, the send
    // error is harmless.
    let _ = cmd_tx.send(EngineCommand::Shutdown);

    // Drop the sender so the engine also sees channel disconnect.
    drop(cmd_tx);

    // Wait for the engine thread. Engine::drop flushes output.
    if let Err(_panic) = engine_handle.join() {
        tracing::error!("engine thread panicked during join");
    } else {
        tracing::info!("engine thread joined cleanly");
    }
}
