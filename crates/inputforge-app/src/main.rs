// Rust guideline compliant 2026-03-07

//! `InputForge`, desktop application entry point.
//!
//! Wires together the engine thread, system tray icon, and optional GUI
//! window. The engine runs on a dedicated thread (`SDL3` is `!Send`), while
//! the main thread owns the tray icon and optionally hosts an `eframe` GUI.

#[cfg(all(feature = "gui-egui", feature = "gui-dioxus"))]
compile_error!("features `gui-egui` and `gui-dioxus` are mutually exclusive");

#[cfg(not(any(feature = "gui-egui", feature = "gui-dioxus")))]
compile_error!("one of `gui-egui` or `gui-dioxus` must be enabled");

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
use inputforge_core::profile::manager::ensure_default_profile;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;
#[cfg(feature = "gui-egui")]
use inputforge_core::state::EngineStatus;

use crate::cli::Cli;
use crate::tray::AppTray;
#[cfg(feature = "gui-egui")]
use crate::tray::TrayAction;

#[cfg(feature = "gui-egui")]
use inputforge_gui::launch_gui;
#[cfg(feature = "gui-dioxus")]
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

    // GUI launch, Shape A: cfg-split because the Dioxus and egui lifecycles
    // diverge. The egui flow is byte-identical to today.

    #[cfg(feature = "gui-dioxus")]
    {
        if let Err(e) = launch_gui(
            Arc::clone(&state),
            cmd_tx.clone(),
            tray.menu_item_ids(),
            settings.clone(),
            cli.start_minimized,
        ) {
            tracing::error!(%e, "GUI exited with error");
        }
        // launch_gui only returns on real Quit (tray Quit click). Fall
        // through to shutdown, no run_tray_loop, no drain_stale_gui_events,
        // no quit_requested flag. The window-hides-on-X behavior is owned
        // by Dioxus via WindowCloseBehaviour::WindowHides set in launch_gui.
    }

    #[cfg(feature = "gui-egui")]
    {
        let mut quit_requested = false;
        if !cli.start_minimized {
            for action in launch_gui_blocking(&tray, &state, &cmd_tx, &settings) {
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
                    TrayAction::ShowGui => {} // already drained, satisfy exhaustiveness
                }
            }
        }
        if !quit_requested {
            run_tray_loop(&tray, &state, &cmd_tx, &settings);
        }
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
// GUI lifecycle
// ---------------------------------------------------------------------------

/// Launch the eframe GUI, blocking until the user closes the window.
///
/// After the window closes, stale `ShowGui` menu events are discarded
/// and any other pending actions (`Quit`, `ToggleActivation`) are
/// returned for the caller to process.
#[cfg(feature = "gui-egui")]
fn launch_gui_blocking(
    tray: &AppTray,
    state: &Arc<RwLock<AppState>>,
    cmd_tx: &mpsc::Sender<EngineCommand>,
    settings: &AppSettings,
) -> Vec<TrayAction> {
    let gui_state = Arc::clone(state);
    let gui_tx = cmd_tx.clone();
    let menu_ids = tray.menu_item_ids();

    if let Err(e) = launch_gui(gui_state, gui_tx, menu_ids, settings.clone(), false) {
        // start_minimized: false, main.rs gates the egui startup launch
        // from cli.start_minimized itself; once we're in launch_gui_blocking,
        // we always want the window visible. Parameter exists only for
        // signature parity with the Dioxus crate (deletes at F16).
        tracing::error!(%e, "GUI exited with error");
    }

    let mut pending = Vec::new();

    // Check if the GUI closed because of a tray Quit click.
    {
        let mut guard = state.write();
        if guard.quit_requested {
            pending.push(TrayAction::Quit);
            guard.quit_requested = false;
        }
    }

    // Drain any events that arrived between the last update() and window close.
    pending.extend(drain_stale_gui_events(tray));
    pending
}

/// Drain queued menu events, discarding stale `ShowGui` clicks.
///
/// While eframe owns the message pump, tray menu clicks accumulate in
/// the `MenuEvent` channel. `ShowGui` events are meaningless (the GUI
/// just closed), but `Quit` and `ToggleActivation` are legitimate user
/// actions that the caller should honor.
#[cfg(feature = "gui-egui")]
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
#[cfg(feature = "gui-egui")]
#[expect(
    unsafe_code,
    reason = "Win32 GetMessageW / TranslateMessage / DispatchMessageW for tray icon message pump"
)]
fn run_tray_loop(
    tray: &AppTray,
    state: &Arc<RwLock<AppState>>,
    cmd_tx: &mpsc::Sender<EngineCommand>,
    settings: &AppSettings,
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
            // WM_QUIT received or error, exit the loop.
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
                    let deferred = launch_gui_blocking(tray, state, cmd_tx, settings);
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
