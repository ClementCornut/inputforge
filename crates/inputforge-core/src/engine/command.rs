// Rust guideline compliant 2026-03-06

use std::path::PathBuf;

use crate::action::Action;
use crate::processing::Calibration;
use crate::types::{DeviceId, InputAddress};

/// Commands sent from the GUI to the engine via an mpsc channel.
#[derive(Debug, PartialEq)]
pub enum EngineCommand {
    /// Load a profile from the given path.
    LoadProfile(PathBuf),
    /// Start processing input events.
    Activate,
    /// Stop processing and flush pending output.
    Deactivate,
    /// Temporarily stop processing without releasing devices.
    Pause,
    /// Resume processing after a pause.
    Resume,
    /// Shut down the engine loop.
    Shutdown,
    /// Set or update a calibration for a specific device axis.
    SetCalibration {
        device: DeviceId,
        axis: u8,
        calibration: Calibration,
    },
    /// Persist current calibrations to the loaded profile file.
    SaveCalibrations,
    /// Set or update a mapping for a specific input and mode.
    SetMapping {
        input: InputAddress,
        mode: String,
        name: Option<String>,
        actions: Vec<Action>,
    },
}
