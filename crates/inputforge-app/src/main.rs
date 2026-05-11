// Rust guideline compliant 2026-03-07

//! `InputForge`, desktop application entry point.
//!
//! Wires together the engine thread, system tray icon, and the Dioxus GUI
//! window. The engine runs on a dedicated thread (`SDL3` is `!Send`).

mod cli;
mod tray;

/// OR the CLI `--start-minimized` flag with the persisted setting.
///
/// Used at the `launch_gui` call site so the existing single-bool flow in
/// `launch_gui` / `LaunchParams.start_minimized` / `lifecycle::apply_start_minimized`
/// stays unchanged. There is no `--no-start-minimized` flag; users with the
/// setting on who want a normal-window launch are an unsupported edge case
/// (no peer surveyed exposes this).
fn resolve_start_minimized(cli_flag: bool, settings_flag: bool) -> bool {
    cli_flag || settings_flag
}

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use clap::Parser;
use mimalloc::MiMalloc;
use parking_lot::RwLock;

use inputforge_core::device::{DeviceHider, NoOpDeviceHider, Sdl3Input};
use inputforge_core::engine::{Engine, EngineCommand};
use inputforge_core::output::mouse::MouseOutput;
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

    // Load settings to read last_profile. The engine re-reads settings
    // itself inside run_engine_inner; this main-side load is for path
    // resolution only.
    let settings = AppSettings::load();

    // Resolve the target profile path. Validate via Profile::load and
    // discard the result: the engine re-reads through
    // EngineCommand::LoadProfile so cold-start and in-session profile
    // switches share one canonical code path (snapshot + prune +
    // last_profile persistence). Validation here preserves today's
    // corrupt-last_profile fallback to default without expanding the
    // EngineCommand surface.
    let profile_path = if let Some(ref path) = cli.profile {
        let _ = Profile::load(path)?;
        path.clone()
    } else {
        match settings.last_profile {
            Some(ref last) if last.exists() => match Profile::load(last) {
                Ok(_) => last.clone(),
                Err(e) => {
                    tracing::warn!(
                        path = %last.display(),
                        %e,
                        "failed to load last-used profile, falling back to default"
                    );
                    let default_path = ensure_default_profile()?;
                    let _ = Profile::load(&default_path)?;
                    default_path
                }
            },
            _ => {
                let default_path = ensure_default_profile()?;
                let _ = Profile::load(&default_path)?;
                default_path
            }
        }
    };

    // Spawn the engine on a dedicated thread. All !Send types (SDL3)
    // are created on this thread.
    let engine_state = Arc::clone(&state);
    let engine_handle = thread::Builder::new()
        .name("engine".into())
        .spawn(move || run_engine(engine_state, cmd_rx))?;

    // Cold-start profile load. Same code path as in-session profile
    // switches: reloads from disk, takes AutoSessionStart snapshot
    // (gated by settings.snapshot.skip_if_unchanged), prunes, refreshes
    // projection rows, persists last_profile.
    cmd_tx.send(EngineCommand::LoadProfile(profile_path))?;

    // Send activate command if requested.
    if cli.enable {
        cmd_tx.send(EngineCommand::Activate)?;
    }

    // Create the system tray icon (always visible).
    let tray = AppTray::new(Arc::clone(&state))?;

    let effective_start_minimized = resolve_start_minimized(
        cli.start_minimized,
        settings.startup.start_minimized_to_tray,
    );

    if let Err(e) = launch_gui(
        Arc::clone(&state),
        cmd_tx.clone(),
        tray.menu_item_ids(),
        tray.toggle_menu_item(),
        effective_start_minimized,
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
    let mouse = Box::new(MouseOutput::new());
    let hider: Box<dyn DeviceHider> = Box::new(NoOpDeviceHider);

    let mut engine = Engine::new(
        input,
        output,
        keyboard,
        mouse,
        hider,
        state,
        commands,
        AppSettings::load(),
        AppSettings::settings_path(),
        inputforge_autostart::new_for_current_platform(),
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

#[cfg(test)]
mod tests {
    use super::resolve_start_minimized;

    #[test]
    fn resolve_start_minimized_or_logic() {
        // (cli, settings) -> expected
        let cases = [
            (false, false, false),
            (true, false, true),
            (false, true, true),
            (true, true, true),
        ];
        for (cli, settings, expected) in cases {
            assert_eq!(
                resolve_start_minimized(cli, settings),
                expected,
                "cli={cli}, settings={settings}"
            );
        }
    }
}
