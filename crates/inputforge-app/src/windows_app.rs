//! Windows desktop runtime wiring.
//!
//! Owns the engine thread, system tray, and Dioxus GUI. Keeping this module
//! target-gated prevents Linux builds from linking Windows desktop runtime
//! dependencies before the Linux backend exists.

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use parking_lot::RwLock;

use inputforge_core::engine::{Engine, EngineCommand};
use inputforge_core::profile::Profile;
use inputforge_core::profile::manager::ensure_default_profile;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;
use inputforge_gui_dx::launch_gui;

use crate::cli::Cli;
use crate::tray::AppTray;

/// Run the Windows engine, tray, and GUI lifecycle.
///
/// # Errors
///
/// Returns an error when profile resolution, thread creation, or command
/// delivery fails.
pub(crate) fn run(cli: Cli) -> Result<()> {
    tracing::info!(?cli, "starting InputForge");

    let state = Arc::new(RwLock::new(AppState::new()));
    let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>();

    // The engine loads settings again inside `run_engine_inner`; this copy is
    // used only to resolve the startup profile and minimized-window policy.
    let settings = AppSettings::load();
    let profile_path = resolve_profile_path(&cli, &settings)?;

    // Construct SDL3 and other !Send backends on their dedicated thread.
    let engine_state = Arc::clone(&state);
    let engine_handle = thread::Builder::new()
        .name("engine".into())
        .spawn(move || run_engine(engine_state, cmd_rx))?;

    // Cold-start profile loading shares the same snapshot and persistence
    // path as an in-session profile switch.
    cmd_tx.send(EngineCommand::LoadProfile(profile_path))?;
    if cli.enable {
        cmd_tx.send(EngineCommand::Activate)?;
    }

    let tray = AppTray::new(Arc::clone(&state))?;
    let effective_start_minimized = resolve_start_minimized(
        cli.start_minimized,
        settings.startup.start_minimized_to_tray,
    );

    if let Err(error) = launch_gui(
        Arc::clone(&state),
        cmd_tx.clone(),
        tray.menu_item_ids(),
        tray.toggle_menu_item(),
        effective_start_minimized,
    ) {
        tracing::error!(%error, "GUI exited with error");
    }

    shutdown(cmd_tx, engine_handle);
    Ok(())
}

fn resolve_profile_path(cli: &Cli, settings: &AppSettings) -> Result<std::path::PathBuf> {
    if let Some(path) = &cli.profile {
        let _ = Profile::load(path)?;
        return Ok(path.clone());
    }

    if let Some(last) = &settings.last_profile
        && last.exists()
    {
        match Profile::load(last) {
            Ok(_) => return Ok(last.clone()),
            Err(error) => {
                tracing::warn!(
                    path = %last.display(),
                    %error,
                    "failed to load last-used profile, falling back to default"
                );
            }
        }
    }

    let default_path = ensure_default_profile()?;
    let _ = Profile::load(&default_path)?;
    Ok(default_path)
}

/// OR the CLI flag with the persisted start-minimized setting.
fn resolve_start_minimized(cli_flag: bool, settings_flag: bool) -> bool {
    cli_flag || settings_flag
}

/// Run the engine on its dedicated thread with panic containment.
fn run_engine(state: Arc<RwLock<AppState>>, commands: mpsc::Receiver<EngineCommand>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_engine_inner(state, commands)
    }));

    match result {
        Ok(Ok(())) => tracing::info!("engine thread exited cleanly"),
        Ok(Err(error)) => tracing::error!(%error, "engine thread exited with error"),
        Err(panic_payload) => {
            let message = panic_payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic_payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("<non-string panic>");
            tracing::error!(panic = message, "engine thread panicked");
        }
    }
}

/// Construct platform backends and enter the engine loop.
fn run_engine_inner(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Receiver<EngineCommand>,
) -> Result<()> {
    let crate::platform::PlatformBackends {
        input,
        controller,
        keyboard,
        mouse,
    } = crate::platform::create()?;
    let mut engine = Engine::new(
        input,
        controller,
        keyboard,
        mouse,
        state,
        commands,
        AppSettings::load(),
        AppSettings::settings_path(),
        inputforge_autostart::new_for_current_platform(),
    );
    engine.run()?;
    Ok(())
}

/// Request engine shutdown and wait for its thread to finish.
fn shutdown(cmd_tx: mpsc::Sender<EngineCommand>, engine_handle: thread::JoinHandle<()>) {
    let _ = cmd_tx.send(EngineCommand::Shutdown);
    drop(cmd_tx);

    if engine_handle.join().is_err() {
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
