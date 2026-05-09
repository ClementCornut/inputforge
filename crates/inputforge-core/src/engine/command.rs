// Rust guideline compliant 2026-04-28

use std::path::PathBuf;

use crate::action::{Action, BulkMapEntry};
use crate::processing::Calibration;
use crate::snapshot::{SnapshotId, SnapshotKind};
use crate::types::{DeviceId, InputAddress};

/// Commands sent from the GUI to the engine via an mpsc channel.
#[derive(Debug, PartialEq)]
pub enum EngineCommand {
    /// Load a profile from the given path.
    LoadProfile(PathBuf),
    /// Create a new library profile and load it.
    CreateProfile { name: String },
    /// Load an external profile without adding it to the library.
    LoadExternalProfileOnce(PathBuf),
    /// Copy an external profile into the library and load it.
    AddExternalProfileToLibrary { path: PathBuf, name: String },
    /// Rename a library profile.
    RenameProfile { old_name: String, new_name: String },
    /// Duplicate a profile into the library without changing the active profile.
    DuplicateProfile { source_path: PathBuf, name: String },
    /// Delete a profile from the library.
    DeleteProfile { name: String },
    /// Reveal a profile path in the OS shell.
    RevealProfile { path: PathBuf },
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

    /// Apply a batch of mapping upserts in a single atomic pass.
    ///
    /// Engine handler order:
    ///   1. Pre-save the in-memory profile to disk (so the snapshot
    ///      captures the user's authored state, not whatever was on
    ///      disk last).
    ///   2. Create an `AutoBeforeBulkMap` snapshot, then `prune`. If
    ///      the snapshot fails, abort: profile is unchanged on disk
    ///      and in memory; a warning is pushed to the warnings
    ///      channel; user retries after fixing the underlying issue.
    ///   3. Run all entries through `Profile::set_mappings_bulk` in
    ///      one in-memory pass.
    ///   4. Save the post-bulk profile to disk.
    ///
    /// `snapshot_label` is the user-visible label attached to the
    /// recovery snapshot. Format guidance:
    /// `"Before bulk-map: <source> to vJoy <id>"`.
    SetMappingsBulk {
        entries: Vec<BulkMapEntry>,
        snapshot_label: String,
    },

    /// Remove the mapping for `(input, mode)`. No-op if no such mapping
    /// exists; the engine handler skips persistence on that fast path.
    RemoveMapping { input: InputAddress, mode: String },

    /// Move the mapping `(input, mode)` to position `target_index_in_group`
    /// within its visual group (Axes / Buttons / Hats). Out-of-bounds
    /// targets clamp to the group's last position; same-position and
    /// single-element-group calls are no-ops with no persistence cost.
    /// Reorder is within-group only; the GUI rejects cross-group drops
    /// before dispatching, so this command never crosses the Axis /
    /// Button / Hat boundary.
    ReorderMapping {
        input: InputAddress,
        mode: String,
        target_index_in_group: usize,
    },

    /// Manually switch the engine to the named mode.
    ///
    /// Idempotent when the engine is already running `mode`. Subsequent
    /// rule-driven mode changes (`Action::ChangeMode` outputs, temporary-mode
    /// pop callbacks) are unaffected: rules win after a manual switch.
    SwitchMode { mode: String },

    /// Re-read `settings.toml` and update in-memory `AppSettings`.
    ///
    /// Snapshot subsystem picks up the new `SnapshotConfig` on the next
    /// command processed. In-flight snapshot operations earlier in the
    /// same `process_commands` drain still see the old config.
    ReloadSettings,

    /// Replace `AppSettings.snapshot` with the supplied config.
    ///
    /// Surgical: replaces only `settings.snapshot`, not other
    /// `AppSettings` fields. Persists `settings.toml` with the new
    /// value; on save failure the in-memory `snapshot` is rolled back
    /// to the pre-command value and a warning is pushed to the
    /// warnings channel.
    ///
    /// When `config.max_count` is *less than* the previous value and a
    /// profile is loaded, the engine prunes the active namespace via
    /// `snapshot::prune_in` (pinned snapshots exempt). Increases never
    /// prune.
    SetSnapshotConfig {
        config: crate::snapshot::SnapshotConfig,
    },

    /// Set or clear the global display alias for a device.
    SetDeviceAlias {
        device: DeviceId,
        alias: Option<String>,
    },

    /// Take a snapshot of the active profile.
    CreateSnapshot {
        kind: SnapshotKind,
        label: Option<String>,
    },

    /// Delete a snapshot by id.
    DeleteSnapshot { id: SnapshotId },

    /// Undo a staged snapshot delete by id.
    UndoSnapshotDelete { id: SnapshotId },

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

    /// Add a new mode under the profile's existing root, or under `parent`
    /// if specified. Default placement: as a child of the root mode.
    AddMode {
        name: String,
        parent: Option<String>,
    },

    /// Rename a mode in the active profile's mode tree, cascading the rename
    /// across all mappings, action graphs, and `startup_mode`.
    RenameMode { from: String, to: String },

    /// Delete a mode and its descendants. Cascade-drops every mapping scoped
    /// to any deleted mode. Errors if the mode is the root or its subtree
    /// contains the profile's startup mode.
    DeleteMode { name: String },

    /// Set the profile's startup mode. Errors if the named mode is not in
    /// the active profile's mode tree.
    SetDefaultMode { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_format_contains_variant_name() {
        let c = EngineCommand::SwitchMode {
            mode: "Combat".to_owned(),
        };
        assert!(format!("{c:?}").contains("SwitchMode"));

        let c = EngineCommand::ReloadSettings;
        assert!(format!("{c:?}").contains("ReloadSettings"));

        let c = EngineCommand::SetDeviceAlias {
            device: DeviceId("dev-1".to_owned()),
            alias: Some("Wheel".to_owned()),
        };
        assert!(format!("{c:?}").contains("SetDeviceAlias"));

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

        let c = EngineCommand::AddMode {
            name: "Combat".to_owned(),
            parent: None,
        };
        assert!(format!("{c:?}").contains("AddMode"));

        let c = EngineCommand::RenameMode {
            from: "Combat".to_owned(),
            to: "Fighter".to_owned(),
        };
        assert!(format!("{c:?}").contains("RenameMode"));

        let c = EngineCommand::DeleteMode {
            name: "Combat".to_owned(),
        };
        assert!(format!("{c:?}").contains("DeleteMode"));

        let c = EngineCommand::SetDefaultMode {
            name: "Combat".to_owned(),
        };
        assert!(format!("{c:?}").contains("SetDefaultMode"));

        let c = EngineCommand::SetSnapshotConfig {
            config: crate::snapshot::SnapshotConfig::default(),
        };
        assert!(format!("{c:?}").contains("SetSnapshotConfig"));
    }

    #[test]
    fn engine_command_derives_debug_partialeq() {
        let a = EngineCommand::AddMode {
            name: "Combat".to_owned(),
            parent: None,
        };
        let b = EngineCommand::AddMode {
            name: "Combat".to_owned(),
            parent: None,
        };
        assert_eq!(a, b, "PartialEq must hold across the new variants");
        let _: String = format!("{a:?}");

        let c = EngineCommand::SetDeviceAlias {
            device: DeviceId("dev-1".to_owned()),
            alias: Some("Wheel".to_owned()),
        };
        assert!(format!("{c:?}").contains("SetDeviceAlias"));
    }

    #[test]
    fn remove_mapping_variant_debug_and_partialeq() {
        use crate::types::{DeviceId, InputId};

        let input = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 3 },
        };
        let a = EngineCommand::RemoveMapping {
            input: input.clone(),
            mode: "Default".to_owned(),
        };
        let b = EngineCommand::RemoveMapping {
            input: input.clone(),
            mode: "Default".to_owned(),
        };
        assert_eq!(a, b, "PartialEq must hold across the new variant");
        assert!(format!("{a:?}").contains("RemoveMapping"));
    }

    #[test]
    fn set_mappings_bulk_variant_debug_and_partialeq() {
        use crate::action::BulkMapEntry;
        use crate::types::{DeviceId, InputId, OutputId, VJoyAxis};

        let entry = BulkMapEntry {
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            output: crate::types::OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        };
        let a = EngineCommand::SetMappingsBulk {
            entries: vec![entry.clone()],
            snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
        };
        let b = EngineCommand::SetMappingsBulk {
            entries: vec![entry],
            snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
        };
        assert_eq!(a, b);
        assert!(format!("{a:?}").contains("SetMappingsBulk"));
    }

    #[test]
    fn set_snapshot_config_variant_debug_and_partialeq() {
        use crate::snapshot::SnapshotConfig;

        let cfg = SnapshotConfig {
            max_count: 25,
            skip_if_unchanged: false,
        };
        let a = EngineCommand::SetSnapshotConfig {
            config: cfg.clone(),
        };
        let b = EngineCommand::SetSnapshotConfig { config: cfg };
        assert_eq!(a, b);
        assert!(format!("{a:?}").contains("SetSnapshotConfig"));
    }
}
