// Rust guideline compliant 2026-03-03

//! Shared application state between engine and GUI.
//!
//! The engine thread owns the mutable state and writes to it each
//! frame. The GUI thread reads through an `Arc<RwLock<AppState>>`
//! reference to display live values.

mod cache;
mod device;
mod status;

pub use cache::InputCacheStore;
pub use device::DeviceState;
pub use status::EngineStatus;

use crate::profile::Profile;

/// Top-level shared state for the application.
///
/// Wrapped in `Arc<RwLock<AppState>>` for thread-safe access
/// between the engine and GUI threads.
#[derive(Debug)]
pub struct AppState {
    /// Connected devices and their live input values.
    pub devices: Vec<DeviceState>,
    /// Name of the currently active mode.
    pub current_mode: String,
    /// Current engine lifecycle status.
    pub engine_status: EngineStatus,
    /// The loaded profile, if any.
    pub active_profile: Option<Profile>,
    /// Cache of the latest value for every physical input.
    pub input_cache: InputCacheStore,
}

impl AppState {
    /// Create a new `AppState` with default values and no profile.
    #[must_use]
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            current_mode: "Default".to_owned(),
            engine_status: EngineStatus::Stopped,
            active_profile: None,
            input_cache: InputCacheStore::new(),
        }
    }

    /// Create a new `AppState` initialized from a profile.
    #[must_use]
    pub fn with_profile(profile: Profile) -> Self {
        let startup_mode = profile.settings().startup_mode().to_owned();
        Self {
            devices: Vec::new(),
            current_mode: startup_mode,
            engine_status: EngineStatus::Stopped,
            active_profile: Some(profile),
            input_cache: InputCacheStore::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_default_values() {
        let state = AppState::new();
        assert!(state.devices.is_empty());
        assert_eq!(state.current_mode, "Default");
        assert_eq!(state.engine_status, EngineStatus::Stopped);
        assert!(state.active_profile.is_none());
    }

    #[test]
    fn app_state_default_trait() {
        let state = AppState::default();
        assert_eq!(state.engine_status, EngineStatus::Stopped);
    }

    #[test]
    fn app_state_debug_format() {
        let state = AppState::new();
        let debug = format!("{state:?}");
        assert!(debug.contains("AppState"));
        assert!(debug.contains("current_mode"));
    }
}
