use std::path::PathBuf;

use inputforge_core::engine::EngineCommand;
use inputforge_core::snapshot::{SnapshotId, SnapshotKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfirmationKind {
    DestructiveF4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToastAction {
    UndoSnapshotDelete { id: SnapshotId },
}

impl ToastAction {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "toast undo commands are exercised through the Profiles action contract"
        )
    )]
    pub(crate) fn command(&self) -> EngineCommand {
        match self {
            Self::UndoSnapshotDelete { id } => EngineCommand::UndoSnapshotDelete { id: *id },
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ProfilesAction {
    pub command: EngineCommand,
    pub confirmation: Option<ConfirmationKind>,
    pub toast_action: Option<ToastAction>,
}

pub(crate) fn profile_open_action(path: PathBuf) -> EngineCommand {
    EngineCommand::LoadProfile(path)
}

pub(crate) fn profile_rename_action(old_name: &str, new_name: &str) -> Option<EngineCommand> {
    let new_name = new_name.trim();
    if new_name.is_empty() || old_name == new_name {
        return None;
    }
    Some(EngineCommand::RenameProfile {
        old_name: old_name.to_owned(),
        new_name: new_name.to_owned(),
    })
}

pub(crate) fn profile_duplicate_action(source_path: PathBuf, name: &str) -> Option<EngineCommand> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some(EngineCommand::DuplicateProfile {
        source_path,
        name: name.to_owned(),
    })
}

pub(crate) fn profile_reveal_action(path: PathBuf) -> EngineCommand {
    EngineCommand::RevealProfile { path }
}

pub(crate) fn create_manual_snapshot_action() -> EngineCommand {
    EngineCommand::CreateSnapshot {
        kind: SnapshotKind::Manual,
        label: None,
    }
}

pub(crate) fn profile_delete_action(name: &str) -> ProfilesAction {
    ProfilesAction {
        command: EngineCommand::DeleteProfile {
            name: name.to_owned(),
        },
        confirmation: Some(ConfirmationKind::DestructiveF4),
        toast_action: None,
    }
}

pub(crate) fn snapshot_delete_action(id: SnapshotId) -> ProfilesAction {
    ProfilesAction {
        command: EngineCommand::DeleteSnapshot { id },
        confirmation: None,
        toast_action: Some(ToastAction::UndoSnapshotDelete { id }),
    }
}

pub(crate) fn snapshot_restore_action(id: SnapshotId) -> ProfilesAction {
    ProfilesAction {
        command: EngineCommand::RestoreSnapshot { id },
        confirmation: Some(ConfirmationKind::DestructiveF4),
        toast_action: None,
    }
}
