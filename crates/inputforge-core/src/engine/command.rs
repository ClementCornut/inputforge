// Rust guideline compliant 2026-04-28

use std::path::PathBuf;

use crate::action::Action;
use crate::processing::Calibration;
use crate::snapshot::{SnapshotId, SnapshotKind};
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

    /// Force the engine into the named mode and pause mode-change rules.
    ///
    /// Idempotent on the same mode (per design decision D15); rotates the
    /// override when called with a different mode.
    ForceMode { mode: String },

    /// Release any active forced-mode override. Current mode is preserved.
    ReleaseMode,

    /// Re-read `settings.toml` and update in-memory `AppSettings`.
    ///
    /// Snapshot subsystem picks up the new `SnapshotConfig` on the next
    /// command processed. In-flight snapshot operations earlier in the
    /// same `process_commands` drain still see the old config.
    ReloadSettings,

    /// Take a snapshot of the active profile.
    CreateSnapshot {
        kind: SnapshotKind,
        label: Option<String>,
    },

    /// Delete a snapshot by id.
    DeleteSnapshot { id: SnapshotId },

    /// Pin or unpin a snapshot.
    PinSnapshot { id: SnapshotId, pinned: bool },

    /// Rename (or clear the label of) a snapshot.
    RenameSnapshot {
        id: SnapshotId,
        label: Option<String>,
    },

    /// Restore the active profile to the named snapshot.
    ///
    /// Engine handler takes an `AutoBeforeRestore` snapshot first;
    /// auto-rolls back to it if the post-restore reload fails (D16).
    RestoreSnapshot { id: SnapshotId },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_format_contains_variant_name() {
        let c = EngineCommand::ForceMode {
            mode: "Combat".to_owned(),
        };
        assert!(format!("{c:?}").contains("ForceMode"));

        let c = EngineCommand::ReleaseMode;
        assert!(format!("{c:?}").contains("ReleaseMode"));

        let c = EngineCommand::ReloadSettings;
        assert!(format!("{c:?}").contains("ReloadSettings"));

        let c = EngineCommand::CreateSnapshot {
            kind: SnapshotKind::Manual,
            label: None,
        };
        assert!(format!("{c:?}").contains("CreateSnapshot"));

        let id = SnapshotId(ulid::Ulid::new());
        assert!(format!("{:?}", EngineCommand::DeleteSnapshot { id }).contains("DeleteSnapshot"));
        assert!(
            format!("{:?}", EngineCommand::PinSnapshot { id, pinned: true })
                .contains("PinSnapshot")
        );
        assert!(
            format!("{:?}", EngineCommand::RenameSnapshot { id, label: None })
                .contains("RenameSnapshot")
        );
        assert!(format!("{:?}", EngineCommand::RestoreSnapshot { id }).contains("RestoreSnapshot"));
    }
}
