//! Captured snapshot actions validate identity before existing operation handlers run.
use super::{Engine, EngineCommand};
use crate::{profile::ProfileId, snapshot::SnapshotKind, state::ActiveSnapshotRow};
use std::path::PathBuf;
/// An existing snapshot operation requested by the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotAction {
    /// Create a manual pinned snapshot without a custom label.
    Manual,
    /// Restore the captured row after identity validation.
    Restore(ActiveSnapshotRow),
    /// Delete the captured row after identity and protection checks.
    Delete(ActiveSnapshotRow),
}
/// Active profile and snapshot identity captured before command delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRequest {
    /// Stable identity of the active profile when the action was requested.
    pub profile: ProfileId,
    /// Lossless native path of that active profile.
    pub path: PathBuf,
    /// Existing operation to execute after checking the captured context.
    pub action: SnapshotAction,
}
impl Engine {
    pub(super) fn snapshot_command(&self, request: SnapshotRequest) -> Option<EngineCommand> {
        let mut state = self.state.write();
        if state
            .active_profile
            .as_ref()
            .is_none_or(|p| p.id() != &request.profile)
            || state.profile_path.as_ref() != Some(&request.path)
        {
            state.warnings.push(
                "Snapshot action cancelled: profile changed. Reopen the action and try again."
                    .into(),
            );
            return None;
        }
        let row = match &request.action {
            SnapshotAction::Manual => None,
            SnapshotAction::Restore(row) | SnapshotAction::Delete(row) => Some(row),
        };
        if row.is_some_and(|row| !state.active_snapshot_rows.contains(row)) {
            state.warnings.push("Snapshot action cancelled: snapshot changed or is no longer available. Reopen the action and try again.".into());
            return None;
        }
        if matches!(&request.action,SnapshotAction::Delete(row) if row.kind==SnapshotKind::AutoSessionStart)
        {
            state
                .warnings
                .push("The session-start recovery snapshot cannot be deleted.".into());
            return None;
        }
        Some(match request.action {
            SnapshotAction::Manual => EngineCommand::CreateSnapshot {
                kind: SnapshotKind::Manual,
                label: None,
            },
            SnapshotAction::Restore(row) => EngineCommand::RestoreSnapshot { id: row.id },
            SnapshotAction::Delete(row) => EngineCommand::DeleteSnapshot { id: row.id },
        })
    }
}
