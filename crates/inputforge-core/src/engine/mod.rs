// Rust guideline compliant 2026-03-06

//! Engine event loop and orchestration.
//!
//! The engine ties together input polling, mode routing, pipeline
//! execution, and output writing into a single event loop that runs
//! on a dedicated thread. Communication with the GUI happens through
//! shared [`AppState`](crate::state::AppState) and an mpsc command
//! channel.

mod command;
mod dependencies;
mod output_handler;
mod run;
#[cfg(all(test, feature = "test-util"))]
mod tests;

pub use command::EngineCommand;
pub use run::MAX_MODE_NAME_GRAPHEMES;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use parking_lot::RwLock;

use crate::callbacks::CallbackRegistry;
use crate::device::traits::{DeviceHider, InputSource};
use crate::mode::ModeState;
use crate::output::traits::{KeyboardSink, OutputSink};
use crate::pipeline::PipelineOutput;
use crate::settings::AppSettings;
use crate::state::AppState;
use crate::types::InputEvent;

/// The main engine that polls input and writes output.
///
/// Constructed with all I/O trait objects and shared state references.
/// Call [`Engine::run`] on a dedicated thread to start the event loop,
/// or [`Engine::tick`] for single-frame testing.
pub struct Engine {
    input: Box<dyn InputSource>,
    output: Box<dyn OutputSink>,
    keyboard: Box<dyn KeyboardSink>,
    #[expect(
        dead_code,
        reason = "will be used for device hiding in activation flow"
    )]
    hider: Box<dyn DeviceHider>,
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Receiver<EngineCommand>,
    callbacks: CallbackRegistry,
    pub(crate) mode_state: ModeState,
    /// Reused across frames to avoid per-frame allocation.
    event_buffer: Vec<InputEvent>,
    /// Reused across frames to batch output cache writes.
    output_buffer: Vec<PipelineOutput>,
    shutdown: bool,
    /// When `true`, the next tick will refresh all cached axis outputs.
    ///
    /// Set on `Activate`/`Resume` so vJoy reflects current physical
    /// device positions immediately, without waiting for a new input event.
    pending_output_refresh: bool,
    /// Application-wide settings; refreshed by `EngineCommand::ReloadSettings`.
    pub(crate) settings: AppSettings,
    /// Disk path the `ReloadSettings` handler reads from. Production passes
    /// `AppSettings::settings_path()`; tests inject a tempdir path so they
    /// don't touch the developer's real `%APPDATA%`.
    pub(crate) settings_path: PathBuf,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("mode_state", &self.mode_state)
            .field("shutdown", &self.shutdown)
            .finish_non_exhaustive()
    }
}

impl Engine {
    /// Create a new engine with the given I/O dependencies.
    ///
    /// Initializes the mode state from the profile's startup mode
    /// if a profile is loaded in `state`, otherwise defaults to
    /// "Default".
    ///
    /// # Thread Safety
    ///
    /// The `Engine` is `!Send` because [`InputSource`] and
    /// [`DeviceHider`] are `!Send` (SDL3 requires same-thread usage).
    /// Construct and call [`run`](Self::run) on the same thread where
    /// the `InputSource` was created.
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "constructor wires every I/O dependency explicitly; a builder \
                  would not improve clarity for this single caller"
    )]
    pub fn new(
        input: Box<dyn InputSource>,
        output: Box<dyn OutputSink>,
        keyboard: Box<dyn KeyboardSink>,
        hider: Box<dyn DeviceHider>,
        state: Arc<RwLock<AppState>>,
        commands: mpsc::Receiver<EngineCommand>,
        settings: AppSettings,
        settings_path: PathBuf,
    ) -> Self {
        let startup_mode = {
            let s = state.read();
            s.active_profile.as_ref().map_or_else(
                || "Default".to_owned(),
                |p| p.settings().startup_mode().to_owned(),
            )
        };

        // Probe the output driver for available virtual devices and publish
        // them to shared state so the GUI can display them.
        let virtual_devices = output.list_devices();
        if !virtual_devices.is_empty() {
            tracing::info!(
                count = virtual_devices.len(),
                "discovered virtual devices from output driver"
            );
            state.write().virtual_devices = virtual_devices;
        }

        {
            let mut state = state.write();
            state.device_aliases.clone_from(&settings.device_aliases);
            state.device_registry.clone_from(&settings.device_registry);
            state.snapshot_config = settings.snapshot.clone();
        };

        let engine = Self {
            input,
            output,
            keyboard,
            hider,
            state,
            commands,
            callbacks: CallbackRegistry::new(),
            mode_state: ModeState::new(startup_mode),
            event_buffer: Vec::with_capacity(64),
            output_buffer: Vec::new(),
            shutdown: false,
            pending_output_refresh: false,
            settings,
            settings_path,
        };

        // Classify the startup-loaded profile's origin if main.rs left
        // it unset. Without this, a fixture loaded from outside the
        // library dir (e.g. a dev `--profile` path) projects with
        // `active_profile_origin == None`, so the GUI's External branch
        // never fires and the row stays invisible. Done before the
        // snapshot refresh so namespace resolution sees the right
        // origin.
        //
        // The path clone is bound to a separate `let` so the read
        // guard's temporary scope ends at the statement terminator,
        // not at the end of the `if let` body. Otherwise the
        // subsequent `engine.state.write()` would deadlock against
        // the still-live read guard (parking_lot's RwLock is not
        // reentrant), and the engine thread would hang inside
        // Engine::new with SDL3 already initialized but no further
        // state updates ever published.
        let startup_profile_path = engine.state.read().profile_path.clone();
        if let Some(path) = startup_profile_path {
            let mut state_guard = engine.state.write();
            if state_guard.active_profile_origin.is_none() {
                state_guard.active_profile_origin = Some(engine.profile_origin_for_path(&path));
            }
        }

        // Populate projection rows from the on-disk library and the active
        // profile's snapshot history so the GUI has data to render before
        // the first command lands. Failures are logged and ignored: a
        // missing library directory is normal on first launch.
        if let Err(e) = engine.refresh_profile_library_rows() {
            tracing::warn!(
                target: "engine",
                error = %e,
                "engine.startup.library_refresh_failed"
            );
        }
        if let Err(e) = engine.refresh_active_snapshot_rows() {
            tracing::warn!(
                target: "engine",
                error = %e,
                "engine.startup.snapshot_refresh_failed"
            );
        }

        engine
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Err(e) = self.output.flush() {
            tracing::error!("Failed to flush output during Engine drop: {e}");
        }
    }
}
