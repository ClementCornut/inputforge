//! Shared controller editing and command delivery for Windows and Linux.
mod axes;
mod edit;
mod policy;
// Preserve the core virtual-device identity range (1..=16).
const MAX_SLOTS: u8 = 16;
#[doc(inline)]
pub use policy::view;

use anyhow::Result;
use dioxus::prelude::Callback;
use inputforge_core::{engine::EngineCommand, state::AppState};
use inputforge_gui_dx::controllers::{Action, Bindings};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};

/// Resolve a global axis action or a profile controller edit against current state.
///
/// Unavailable or obsolete operations produce no command; core owns execution.
/// # Errors
/// Returns malformed field input or core configuration validation errors.
pub fn resolve(state: &AppState, action: Action) -> Result<Option<EngineCommand>> {
    match action {
        Action::Polarity {
            device,
            axis,
            code,
            value,
        } => axes::resolve(state, device, axis, code, Some(&value)),
        Action::Detect { device, axis, code } => axes::resolve(state, device, axis, code, None),
        action => edit::resolve(state, action),
    }
}

/// Create editor-scoped callbacks for controller operations.
#[must_use]
pub fn bindings(state: Arc<RwLock<AppState>>, commands: mpsc::Sender<EngineCommand>) -> Bindings {
    Bindings {
        dispatch: Callback::new(move |action| match dispatch(action, &state, &commands) {
            Ok(()) => None,
            Err(error) => {
                tracing::error!(name: "controllers.dispatch.failed", %error, "controller operation failed");
                Some(error.to_string())
            }
        }),
    }
}

/// Recheck and deliver a controller command without holding the engine lock during delivery.
///
/// # Errors
/// Returns input/configuration validation errors or failed command delivery.
pub fn dispatch(
    action: Action,
    state: &RwLock<AppState>,
    commands: &mpsc::Sender<EngineCommand>,
) -> Result<()> {
    let command = resolve(&state.read(), action)?;
    if let Some(command) = command {
        commands.send(command)?;
    }
    Ok(())
}
