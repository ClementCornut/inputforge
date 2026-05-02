// Rust guideline compliant 2026-03-06

//! Shared application state between engine and GUI.
//!
//! The engine thread owns the mutable state and writes to it each
//! frame. The GUI thread reads through an `Arc<RwLock<AppState>>`
//! reference to display live values.

mod cache;
mod calibration;
mod device;
mod output_cache;
mod status;

pub use cache::{InputCacheEntry, InputCacheStore};
pub use calibration::DeviceCalibrationStore;
pub use device::DeviceState;
pub use output_cache::OutputCacheStore;
pub use status::EngineStatus;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::profile::Profile;
use crate::types::VirtualDeviceConfig;

/// Sticky forced-mode override.
///
/// While `Some` on `AppState`, mode-change rules are paused: pipeline
/// `Action::ChangeMode` outputs and `ReleaseCallback::PopTemporaryMode`
/// are no-ops. Cleared by `EngineCommand::ReleaseMode` or by a
/// `LoadProfile` (which always resets the override).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForcedMode {
    /// Name of the mode held by the override.
    pub mode: String,
}

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
    /// Cache of the latest values written to virtual vJoy outputs.
    pub output_cache: OutputCacheStore,
    /// Discovered virtual vJoy device configurations.
    ///
    /// Populated by the engine when it probes the vJoy driver at startup.
    /// Empty until the driver is queried.
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    /// Per-device, per-axis calibration configurations.
    pub calibrations: DeviceCalibrationStore,
    /// File path of the currently loaded profile, if loaded from disk.
    pub profile_path: Option<PathBuf>,
    /// Warnings surfaced to the user (e.g., `HidHide` unavailable).
    pub warnings: Vec<String>,
    /// When `Some`, the engine is in a forced-mode override; mode-change
    /// rules are paused.
    pub mode_force: Option<ForcedMode>,
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
            output_cache: OutputCacheStore::new(),
            virtual_devices: Vec::new(),
            calibrations: DeviceCalibrationStore::new(),
            profile_path: None,
            warnings: Vec::new(),
            mode_force: None,
        }
    }

    /// Create a new `AppState` initialized from a profile.
    ///
    /// Populates calibrations from the profile's calibration entries.
    /// Invalid entries are skipped with a warning.
    #[must_use]
    pub fn with_profile(profile: Profile) -> Self {
        let startup_mode = profile.settings().startup_mode().to_owned();
        let mut calibrations = DeviceCalibrationStore::new();
        for entry in profile.calibrations() {
            match entry.to_calibration() {
                Ok(cal) => {
                    calibrations.set(entry.device.clone(), entry.axis, cal);
                }
                Err(e) => {
                    tracing::warn!(
                        device = %entry.device.0,
                        axis = entry.axis,
                        error = %e,
                        "skipping invalid calibration entry in with_profile"
                    );
                }
            }
        }
        Self {
            devices: Vec::new(),
            current_mode: startup_mode,
            engine_status: EngineStatus::Stopped,
            active_profile: Some(profile),
            input_cache: InputCacheStore::new(),
            output_cache: OutputCacheStore::new(),
            virtual_devices: Vec::new(),
            calibrations,
            profile_path: None,
            warnings: Vec::new(),
            mode_force: None,
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

    #[test]
    fn app_state_new_mode_force_is_none() {
        let state = AppState::new();
        assert!(state.mode_force.is_none());
    }

    #[test]
    fn forced_mode_serde_round_trip() {
        let f = ForcedMode {
            mode: "Combat".to_owned(),
        };
        let s = toml::to_string(&f).unwrap();
        let back: ForcedMode = toml::from_str(&s).unwrap();
        assert_eq!(f, back);
    }
}
