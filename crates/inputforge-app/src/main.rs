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

use inputforge_core::device::{DeviceHider, HidHideManager, NoOpDeviceHider, Sdl3Input};
use inputforge_core::engine::{Engine, EngineCommand};
use inputforge_core::output::{KeyboardOutput, VJoyOutput};
use inputforge_core::profile::Profile;
use inputforge_core::profile::manager::{ensure_default_profile, list_profiles_in};
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, ProfileLibraryRow, ProfileOrigin};

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

    // Pre-populate origin and library rows on the main thread so the
    // Profiles panel renders correct projections immediately, even if
    // the engine thread is slow to initialize or fails to acquire its
    // I/O resources. The engine's Engine::new repeats this work as a
    // refresh once it spins up; the writes are idempotent.
    populate_initial_library_state(&state);

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
    // through to shutdown, no run_tray_loop, no drain_stale_gui_events.
    // The window-hides-on-X behavior is owned by Dioxus via
    // WindowCloseBehaviour::WindowHides set in launch_gui.

    // Graceful shutdown.
    shutdown(cmd_tx, engine_handle);

    Ok(())
}

// ---------------------------------------------------------------------------
// Initial library state
// ---------------------------------------------------------------------------

/// Populate `active_profile_origin` and `profile_library_rows` from
/// the on-disk library directory before the engine thread spins up.
///
/// Without this, a fresh `AppState::with_profile(...)` leaves origin
/// as `None` and library rows empty until the engine refreshes them.
/// If the engine thread fails to acquire `SDL3` / `vJoy` / `HidHide`
/// resources, the GUI would otherwise show the active profile
/// without an `External` badge and an empty library list.
///
/// Best-effort: filesystem failures log and produce defaults.
fn populate_initial_library_state(state: &Arc<RwLock<AppState>>) {
    let library_dir = AppSettings::profiles_dir();

    let mut guard = state.write();

    // Origin classification.
    if guard.active_profile_origin.is_none()
        && let Some(path) = guard.profile_path.clone()
    {
        let origin = if path.starts_with(&library_dir) {
            ProfileOrigin::Library
        } else {
            ProfileOrigin::External
        };
        guard.active_profile_origin = Some(origin);
    }

    // Library row scan.
    let active_path = guard.profile_path.clone();
    match list_profiles_in(&library_dir) {
        Ok(profiles) => {
            let rows: Vec<ProfileLibraryRow> = profiles
                .into_iter()
                .map(|profile| {
                    let is_active = active_path
                        .as_ref()
                        .is_some_and(|active| active == &profile.path);
                    let (mode_count, last_edited_at) = read_profile_metadata(&profile.path);
                    ProfileLibraryRow {
                        name: profile.name,
                        path: profile.path,
                        origin: ProfileOrigin::Library,
                        is_active,
                        mode_count,
                        last_edited_at,
                    }
                })
                .collect();
            guard.profile_library_rows = rows;
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                library_dir = %library_dir.display(),
                "failed to scan profile library at startup"
            );
        }
    }
}

/// Read mode count and last-edit timestamp for a profile file. Mirrors
/// the engine-side helper so main.rs can populate library rows without
/// crossing crate visibility boundaries.
fn read_profile_metadata(path: &std::path::Path) -> (u32, Option<chrono::DateTime<chrono::Utc>>) {
    let mode_count = Profile::load(path)
        .map(|p| u32::try_from(p.modes().all_modes().len()).unwrap_or(u32::MAX))
        .unwrap_or(0);
    let last_edited_at = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(chrono::DateTime::<chrono::Utc>::from);
    (mode_count, last_edited_at)
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
