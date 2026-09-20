//! Shared desktop lifecycle. Backends are constructed only on the engine thread.
use crate::cli::Cli;
use anyhow::Result;
use inputforge_core::{
    engine::EngineCommand,
    profile::{Profile, manager::ensure_default_profile},
    settings::AppSettings,
    state::AppState,
};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};
use std::{cell::RefCell, rc::Rc, thread};

pub(crate) fn run(cli: &Cli) -> Result<()> {
    let settings = AppSettings::load();
    let profile = resolve_profile_path(cli, &settings)?;
    let minimized = resolve_start_minimized(
        cli.start_minimized,
        settings.startup.start_minimized_to_tray,
    );
    desktop_session(
        profile,
        cli.enable,
        minimized,
        crate::engine_thread::run_engine,
        launch,
    )
}

fn desktop_session(
    profile: std::path::PathBuf,
    enable: bool,
    minimized: bool,
    run_engine: impl FnOnce(Arc<RwLock<AppState>>, mpsc::Receiver<EngineCommand>) + Send + 'static,
    launch_gui: impl FnOnce(
        Arc<RwLock<AppState>>,
        mpsc::Sender<EngineCommand>,
        bool,
        Box<dyn FnMut()>,
    ) -> Result<()>,
) -> Result<()> {
    let state = Arc::new(RwLock::new(AppState::new()));
    let (commands, receiver) = mpsc::channel();
    let engine_state = Arc::clone(&state);
    let engine = thread::Builder::new()
        .name("engine".into())
        .spawn(move || run_engine(engine_state, receiver))?;
    let engine = Rc::new(RefCell::new(Some(engine)));
    let shutdown_handle = Rc::clone(&engine);
    let shutdown_commands = commands.clone();
    let result = (|| {
        commands.send(EngineCommand::LoadProfile(profile))?;
        if enable {
            commands.send(EngineCommand::Activate)?;
        }
        launch_gui(
            state,
            commands.clone(),
            minimized,
            Box::new(move || {
                stop_engine(&shutdown_commands, &shutdown_handle);
            }),
        )
    })();
    stop_engine(&commands, &engine);
    result
}

fn stop_engine(
    commands: &mpsc::Sender<EngineCommand>,
    engine: &RefCell<Option<thread::JoinHandle<()>>>,
) {
    let Some(handle) = engine.borrow_mut().take() else {
        return;
    };
    let _ = commands.send(EngineCommand::Shutdown);
    if handle.join().is_err() {
        tracing::error!("engine thread panicked during join");
    }
}

#[cfg(target_os = "windows")]
fn launch(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    minimized: bool,
    on_exit: Box<dyn FnMut()>,
) -> Result<()> {
    let tray = crate::tray::AppTray::new(Arc::clone(&state))?;
    inputforge_gui_dx::launch_gui(
        state,
        commands,
        tray.menu_item_ids(),
        tray.toggle_menu_item(),
        minimized,
        on_exit,
    )
}
#[cfg(target_os = "linux")]
fn launch(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    minimized: bool,
    on_exit: Box<dyn FnMut()>,
) -> Result<()> {
    if minimized {
        state.write().session.notice =
            Some("Start minimized is unavailable on Linux; the editor opens visibly.".into());
    }
    inputforge_gui_dx::launch_gui_without_tray(state, commands, on_exit)
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_failure_still_sends_shutdown_and_joins_engine() {
        let recorded = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let received = Arc::clone(&recorded);
        let result = desktop_session(
            "fixture.toml".into(),
            true,
            false,
            move |_, commands| {
                for command in commands {
                    let quit = command == EngineCommand::Shutdown;
                    received.lock().push(command);
                    if quit {
                        break;
                    }
                }
            },
            |_, _, _, _| Err(anyhow::anyhow!("launch failed")),
        );
        assert!(result.is_err());
        assert_eq!(
            *recorded.lock(),
            [
                EngineCommand::LoadProfile("fixture.toml".into()),
                EngineCommand::Activate,
                EngineCommand::Shutdown
            ]
        );
    }
    #[test]
    fn event_loop_exit_callback_finishes_engine_before_returning() {
        let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let engine_finished = Arc::clone(&finished);
        let observed = Arc::clone(&finished);
        desktop_session(
            "fixture.toml".into(),
            false,
            false,
            move |_, commands| {
                while let Ok(command) = commands.recv() {
                    if command == EngineCommand::Shutdown {
                        break;
                    }
                }
                engine_finished.store(true, std::sync::atomic::Ordering::SeqCst);
            },
            move |_, _, _, mut on_exit| {
                on_exit();
                // Return an error on failure so orchestration still tears down the test engine.
                anyhow::ensure!(
                    observed.load(std::sync::atomic::Ordering::SeqCst),
                    "event-loop exit raced engine cleanup"
                );
                Ok(())
            },
        )
        .unwrap();
        assert!(finished.load(std::sync::atomic::Ordering::SeqCst));
    }
}
