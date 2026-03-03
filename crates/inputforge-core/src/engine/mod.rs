// Rust guideline compliant 2026-03-03

//! Engine event loop and orchestration.
//!
//! The engine ties together input polling, mode routing, pipeline
//! execution, and output writing into a single event loop that runs
//! on a dedicated thread. Communication with the GUI happens through
//! shared [`AppState`](crate::state::AppState) and an mpsc command
//! channel.

mod command;
mod output_handler;
mod run;
#[cfg(all(test, feature = "test-util"))]
mod tests;

pub use command::EngineCommand;

use std::sync::Arc;
use std::sync::mpsc;

use parking_lot::RwLock;

use crate::callbacks::CallbackRegistry;
use crate::device::traits::{DeviceHider, InputSource};
use crate::mode::ModeState;
use crate::output::traits::{KeyboardSink, OutputSink};
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
    mode_state: ModeState,
    /// Reused across frames to avoid per-frame allocation.
    event_buffer: Vec<InputEvent>,
    shutdown: bool,
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
    pub fn new(
        input: Box<dyn InputSource>,
        output: Box<dyn OutputSink>,
        keyboard: Box<dyn KeyboardSink>,
        hider: Box<dyn DeviceHider>,
        state: Arc<RwLock<AppState>>,
        commands: mpsc::Receiver<EngineCommand>,
    ) -> Self {
        let startup_mode = {
            let s = state.read();
            s.active_profile.as_ref().map_or_else(
                || "Default".to_owned(),
                |p| p.settings().startup_mode().to_owned(),
            )
        };

        Self {
            input,
            output,
            keyboard,
            hider,
            state,
            commands,
            callbacks: CallbackRegistry::new(),
            mode_state: ModeState::new(startup_mode),
            event_buffer: Vec::with_capacity(64),
            shutdown: false,
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Err(e) = self.output.flush() {
            tracing::error!("Failed to flush output during Engine drop: {e}");
        }
    }
}
