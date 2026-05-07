use std::path::PathBuf;

use inputforge_core::engine::EngineCommand;
use inputforge_core::snapshot::{SnapshotId, SnapshotKind};

/// Filesystem-illegal characters mirrored from
/// `inputforge_core::profile::manager::ILLEGAL_CHARS`. Centralizing the
/// list in the GUI lets us reject names inline before dispatching a
/// command, so the user sees the same rejection messages whether the
/// engine returns or the GUI catches them.
const ILLEGAL_NAME_CHARS: &[char] = &[':', '\\', '/', '*', '?', '"', '<', '>', '|'];

/// Reasons a New Profile name or rename can be rejected before any
/// command is dispatched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NewProfileValidationError {
    EmptyName,
    IllegalCharacter(char),
    DuplicateName,
    MissingPath,
}

impl NewProfileValidationError {
    pub(crate) fn user_message(&self) -> String {
        match self {
            Self::EmptyName => "Name cannot be empty.".to_owned(),
            Self::IllegalCharacter(c) => format!("Name cannot contain '{c}'."),
            Self::DuplicateName => "A profile with this name already exists.".to_owned(),
            Self::MissingPath => "Pick a profile file first.".to_owned(),
        }
    }
}

/// Validate a new profile name against the user's library.
///
/// Returns the trimmed name on success.
pub(crate) fn validate_new_profile_name(
    name: &str,
    existing_names: &[String],
) -> Result<String, NewProfileValidationError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(NewProfileValidationError::EmptyName);
    }
    if let Some(c) = trimmed.chars().find(|c| ILLEGAL_NAME_CHARS.contains(c)) {
        return Err(NewProfileValidationError::IllegalCharacter(c));
    }
    if existing_names
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(trimmed))
    {
        return Err(NewProfileValidationError::DuplicateName);
    }
    Ok(trimmed.to_owned())
}

/// Validate an inline rename. Case-only renames (e.g., "Alpha" -> "ALPHA")
/// are accepted even when the name appears in `existing_names`, since the
/// engine routes them as a same-row rename.
pub(crate) fn validate_rename(
    old: &str,
    new: &str,
    existing_names: &[String],
) -> Result<String, NewProfileValidationError> {
    let trimmed = new.trim();
    if trimmed.is_empty() {
        return Err(NewProfileValidationError::EmptyName);
    }
    if let Some(c) = trimmed.chars().find(|c| ILLEGAL_NAME_CHARS.contains(c)) {
        return Err(NewProfileValidationError::IllegalCharacter(c));
    }
    if !trimmed.eq_ignore_ascii_case(old)
        && existing_names
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(trimmed))
    {
        return Err(NewProfileValidationError::DuplicateName);
    }
    Ok(trimmed.to_owned())
}

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
