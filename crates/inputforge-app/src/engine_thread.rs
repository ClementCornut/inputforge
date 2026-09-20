use anyhow::Result;
use inputforge_core::{
    engine::{Engine, EngineCommand},
    settings::AppSettings,
    state::AppState,
};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};

/// Run the engine on its dedicated thread with panic containment.
#[expect(
    clippy::needless_pass_by_value,
    reason = "engine thread owns this state handle for its lifetime"
)]
pub(crate) fn run_engine(state: Arc<RwLock<AppState>>, commands: mpsc::Receiver<EngineCommand>) {
    run_guarded(&state, || run_engine_inner(Arc::clone(&state), commands));
}

fn run_guarded(state: &Arc<RwLock<AppState>>, run: impl FnOnce() -> Result<()>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
    let mut published = state.write();
    published.session.offline = true;
    published.session.ready = false;
    published.session.captured.clear();
    published.session.output_active = false;
    published.engine_status = inputforge_core::state::EngineStatus::Stopped;

    match result {
        Ok(Ok(())) => tracing::info!("engine thread exited cleanly"),
        Ok(Err(error)) => {
            published.engine_status = inputforge_core::state::EngineStatus::Faulted;
            published.session.error =
                Some(format!("Engine exited: {error}. Restart the application."));
            tracing::error!(%error, "engine thread exited with error");
        }
        Err(panic_payload) => {
            let message = panic_payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic_payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("<non-string panic>");
            published.engine_status = inputforge_core::state::EngineStatus::Faulted;
            published.session.error = Some(format!(
                "Engine panicked: {message}. Restart the application."
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_or_panicked_engine_is_never_published_as_running() {
        for panic in [false, true] {
            let state = Arc::new(RwLock::new(AppState::new()));
            state.write().engine_status = inputforge_core::state::EngineStatus::Running;
            run_guarded(&state, || {
                assert!(!panic, "injected panic");
                Err(anyhow::anyhow!("injected backend failure"))
            });
            assert!(state.read().session.offline);
            assert_eq!(
                state.read().engine_status,
                inputforge_core::state::EngineStatus::Faulted
            );
            assert!(state.read().session.error.is_some());
        }
    }
}
