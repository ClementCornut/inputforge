#![cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into row controls in later tasks")
)]

use inputforge_core::engine::EngineCommand;
use inputforge_core::snapshot::SnapshotId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfirmationKind {
    DestructiveF4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToastAction {
    UndoSnapshotDelete { id: SnapshotId },
}

impl ToastAction {
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
