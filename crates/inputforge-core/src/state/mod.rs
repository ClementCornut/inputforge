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

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::profile::Profile;
use crate::settings::DeviceRecord;
use crate::types::{DeviceId, VirtualDeviceConfig};

/// Origin of the currently loaded profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileOrigin {
    /// Profile is stored in the app profile library.
    Library,
    /// Profile was loaded from an arbitrary external path.
    External,
}

/// Engine-projected profile library row for UI presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileLibraryRow {
    /// Profile display name.
    pub name: String,
    /// Absolute profile path.
    pub path: PathBuf,
    /// Profile origin.
    pub origin: ProfileOrigin,
    /// Whether this row is the active profile.
    pub is_active: bool,
    /// Number of modes declared by the profile.
    ///
    /// Projected by the engine from `profile.modes().all_modes().len()`.
    /// Defaults to `0` when the profile cannot be loaded for projection.
    pub mode_count: u32,
    /// Last filesystem modification time of the profile file.
    ///
    /// Projected from `std::fs::metadata(path)?.modified()?`. `None` when
    /// the platform does not expose mtime, when the conversion to UTC fails,
    /// or when the metadata read fails.
    pub last_edited_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Engine-projected snapshot row for the active profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSnapshotRow {
    /// Snapshot identifier.
    pub id: crate::snapshot::SnapshotId,
    /// Snapshot creation kind.
    pub kind: crate::snapshot::SnapshotKind,
    /// Optional user-facing label.
    pub label: Option<String>,
    /// Snapshot creation timestamp.
    pub taken_at: chrono::DateTime<chrono::Utc>,
    /// Whether the snapshot is pinned.
    pub pinned: bool,
}

/// Top-level shared state for the application.
///
/// Wrapped in `Arc<RwLock<AppState>>` for thread-safe access
/// between the engine and GUI threads.
#[derive(Debug)]
pub struct AppState {
    /// Connected devices and their live input values.
    pub devices: Vec<DeviceState>,
    /// App-wide custom device aliases mirrored from `AppSettings`.
    pub device_aliases: HashMap<DeviceId, String>,
    /// Last-known physical device records mirrored from `AppSettings`.
    pub device_registry: HashMap<DeviceId, DeviceRecord>,
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
    /// Origin of the currently loaded profile, if any.
    pub active_profile_origin: Option<ProfileOrigin>,
    /// Engine-projected rows for the profile library.
    pub profile_library_rows: Vec<ProfileLibraryRow>,
    /// Engine-projected snapshot rows for the active profile.
    pub active_snapshot_rows: Vec<ActiveSnapshotRow>,
    /// Warnings surfaced to the user (e.g., `HidHide` unavailable).
    pub warnings: Vec<String>,
}

impl AppState {
    /// Create a new `AppState` with default values and no profile.
    #[must_use]
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            device_aliases: HashMap::new(),
            device_registry: HashMap::new(),
            current_mode: "Default".to_owned(),
            engine_status: EngineStatus::Stopped,
            active_profile: None,
            input_cache: InputCacheStore::new(),
            output_cache: OutputCacheStore::new(),
            virtual_devices: Vec::new(),
            calibrations: DeviceCalibrationStore::new(),
            profile_path: None,
            active_profile_origin: None,
            profile_library_rows: Vec::new(),
            active_snapshot_rows: Vec::new(),
            warnings: Vec::new(),
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
            device_aliases: HashMap::new(),
            device_registry: HashMap::new(),
            current_mode: startup_mode,
            engine_status: EngineStatus::Stopped,
            active_profile: Some(profile),
            input_cache: InputCacheStore::new(),
            output_cache: OutputCacheStore::new(),
            virtual_devices: Vec::new(),
            calibrations,
            profile_path: None,
            active_profile_origin: None,
            profile_library_rows: Vec::new(),
            active_snapshot_rows: Vec::new(),
            warnings: Vec::new(),
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
