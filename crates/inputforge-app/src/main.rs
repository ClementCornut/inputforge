// Rust guideline compliant 2026-03-06

//! `InputForge` — desktop application entry point.
//!
//! Wires together the engine thread, system tray icon, and optional GUI
//! window. The engine runs on a dedicated thread (`SDL3` is `!Send`), while
//! the main thread owns the tray icon and optionally hosts an `eframe` GUI.

mod cli;
mod tray;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use clap::Parser;
use mimalloc::MiMalloc;
use parking_lot::RwLock;

use inputforge_core::device::{DeviceHider, HidHideManager, NoOpDeviceHider, Sdl3Input};
use inputforge_core::engine::{Engine, EngineCommand};
use inputforge_core::output::{KeyboardOutput, VJoyOutput};
use inputforge_core::profile::Profile;
use inputforge_core::state::AppState;
use inputforge_core::state::EngineStatus;

use crate::cli::Cli;
use crate::tray::{AppTray, TrayAction};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    tracing::info!(?cli, "starting InputForge");

    // Shared state and command channel.
    let state = Arc::new(RwLock::new(AppState::new()));
    let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>();

    // Load profile from CLI if provided.
    if let Some(ref path) = cli.profile {
        let profile = Profile::load(path)?;
        let mut guard = state.write();
        *guard = AppState::with_profile(profile);
        guard.profile_path = Some(path.clone());
    }

    // Spawn the engine on a dedicated thread. All !Send types (SDL3,
    // HidHide) are created on this thread.
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

    // Launch the GUI immediately unless --start-minimized.
    let mut quit_requested = false;
    if !cli.start_minimized {
        for action in launch_gui_blocking(&tray, &state, &cmd_tx) {
            match action {
                TrayAction::Quit => quit_requested = true,
                TrayAction::ToggleActivation => {
                    let status = state.read().engine_status;
                    let cmd = match status {
                        EngineStatus::Running => EngineCommand::Deactivate,
                        EngineStatus::Paused | EngineStatus::Stopped => EngineCommand::Activate,
                    };
                    let _ = cmd_tx.send(cmd);
                }
                TrayAction::ShowGui => {} // already drained, but satisfy exhaustiveness
            }
        }
    }

    // Run the tray event loop until the user selects Quit.
    if !quit_requested {
        run_tray_loop(&tray, &state, &cmd_tx);
    }

    // Graceful shutdown.
    shutdown(cmd_tx, engine_handle);

    Ok(())
}

// ---------------------------------------------------------------------------
// Engine thread
// ---------------------------------------------------------------------------

/// Run the engine on a dedicated thread.
///
/// Creates all `!Send` I/O types here (`SDL3`, `HidHide`) and enters the
/// engine loop. Wrapped in [`std::panic::catch_unwind`] so that `Drop`
/// impls (`HidHide` unhide, `vJoy` flush) still run on panic.
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
    let hider: Box<dyn DeviceHider> = match HidHideManager::new() {
        Ok(h) => Box::new(h),
        Err(e) => {
            tracing::warn!(%e, "HidHide unavailable, device hiding disabled");
            state.write().warnings.push(
                "HidHide unavailable \u{2014} device hiding is disabled. \
                 Run InputForge as administrator to enable HidHide."
                    .into(),
            );
            Box::new(NoOpDeviceHider)
        }
    };

    let mut engine = Engine::new(input, output, keyboard, hider, state, commands);
    engine.run()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// GUI lifecycle
// ---------------------------------------------------------------------------

/// Launch the eframe GUI, blocking until the user closes the window.
///
/// After the window closes, stale `ShowGui` menu events are discarded
/// and any other pending actions (`Quit`, `ToggleActivation`) are
/// returned for the caller to process.
fn launch_gui_blocking(
    tray: &AppTray,
    state: &Arc<RwLock<AppState>>,
    cmd_tx: &mpsc::Sender<EngineCommand>,
) -> Vec<TrayAction> {
    let gui_state = Arc::clone(state);
    let gui_tx = cmd_tx.clone();

    if let Err(e) = inputforge_gui::launch_gui(gui_state, gui_tx) {
        tracing::error!(%e, "GUI exited with error");
    }

    drain_stale_gui_events(tray)
}

/// Drain queued menu events, discarding stale `ShowGui` clicks.
///
/// While eframe owns the message pump, tray menu clicks accumulate in
/// the `MenuEvent` channel. `ShowGui` events are meaningless (the GUI
/// just closed), but `Quit` and `ToggleActivation` are legitimate user
/// actions that the caller should honor.
fn drain_stale_gui_events(tray: &AppTray) -> Vec<TrayAction> {
    let mut pending = Vec::new();
    while let Some(action) = tray.poll_event() {
        if action != TrayAction::ShowGui {
            pending.push(action);
        }
    }
    pending
}

// ---------------------------------------------------------------------------
// Tray event loop
// ---------------------------------------------------------------------------

/// Run the Win32 message pump for tray icon events.
///
/// Blocks on `GetMessageW` (zero CPU when idle). Processes tray menu
/// actions until the user selects Quit.
#[expect(
    unsafe_code,
    reason = "Win32 GetMessageW / TranslateMessage / DispatchMessageW for tray icon message pump"
)]
fn run_tray_loop(
    tray: &AppTray,
    state: &Arc<RwLock<AppState>>,
    cmd_tx: &mpsc::Sender<EngineCommand>,
) {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, TranslateMessage,
    };

    loop {
        tray.refresh_toggle_label();

        // Block until a Windows message arrives. Tray menu clicks,
        // system messages, and WM_QUIT all wake this call.
        let mut msg = MSG::default();

        // SAFETY: `GetMessageW` is a standard Win32 blocking call.
        // Passing `None` for hwnd processes messages for all windows
        // owned by this thread (including the tray icon's hidden window).
        let ret = unsafe { GetMessageW(&raw mut msg, None, 0, 0) };
        if ret.0 <= 0 {
            // WM_QUIT received or error — exit the loop.
            break;
        }
        // SAFETY: `TranslateMessage` and `DispatchMessageW` are standard
        // Win32 message dispatch calls operating on a valid `MSG` that
        // was populated by `GetMessageW` above.
        let _ = unsafe { TranslateMessage(&raw const msg) };
        // SAFETY: See comment above.
        unsafe { DispatchMessageW(&raw const msg) };

        // Process all queued tray menu events after each message pump cycle.
        while let Some(action) = tray.poll_event() {
            match action {
                TrayAction::ShowGui => {
                    let deferred = launch_gui_blocking(tray, state, cmd_tx);
                    tray.refresh_toggle_label();
                    for deferred_action in deferred {
                        match deferred_action {
                            TrayAction::Quit => return,
                            TrayAction::ToggleActivation => {
                                let status = state.read().engine_status;
                                let cmd = match status {
                                    EngineStatus::Running => EngineCommand::Deactivate,
                                    EngineStatus::Paused | EngineStatus::Stopped => {
                                        EngineCommand::Activate
                                    }
                                };
                                let _ = cmd_tx.send(cmd);
                                tray.refresh_toggle_label();
                            }
                            TrayAction::ShowGui => {}
                        }
                    }
                }
                TrayAction::ToggleActivation => {
                    let status = state.read().engine_status;
                    let cmd = match status {
                        EngineStatus::Running => EngineCommand::Deactivate,
                        EngineStatus::Paused | EngineStatus::Stopped => EngineCommand::Activate,
                    };
                    let _ = cmd_tx.send(cmd);
                    tray.refresh_toggle_label();
                }
                TrayAction::Quit => return,
            }
        }
    }
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

    // Wait for the engine thread. Engine::drop flushes output;
    // HidHideManager::drop restores hidden devices.
    if let Err(_panic) = engine_handle.join() {
        tracing::error!("engine thread panicked during join");
    } else {
        tracing::info!("engine thread joined cleanly");
    }
}
