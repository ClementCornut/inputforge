//! Snapshot identity, confirmation and drawer transitions.
use super::policy;
use inputforge_core::{
    engine::{EngineCommand, SnapshotAction, SnapshotRequest},
    snapshot::SnapshotKind,
    state::AppState,
};
use inputforge_gui_dx::profiles::{
    Target as Profile, View,
    snapshots::{Intent, Row, Target},
};
fn active(profile: &Profile, state: &AppState) -> Result<(), String> {
    policy::current(profile, state)?;
    if profile.profile.is_none() || state.profile_path.as_ref() != Some(&profile.path) {
        return Err("This action requires the active profile.".into());
    }
    Ok(())
}
fn current(target: &Target, state: &AppState) -> Result<(), String> {
    active(&target.profile, state)?;
    if !state.active_snapshot_rows.contains(&target.snapshot) {
        return Err(
            "Snapshot changed or is no longer available. Cancel and reopen this action.".into(),
        );
    }
    Ok(())
}
pub(super) fn project(state: &AppState, view: &mut View) {
    let profile = state
        .profile_path
        .as_ref()
        .filter(|_| state.active_profile.is_some())
        .and_then(|path| policy::target(path, state).ok());
    let rows = crate::bridge::meta::snapshot_rows(state);
    view.snapshots.rows = profile
        .as_ref()
        .map(|profile| {
            state
                .active_snapshot_rows
                .iter()
                .zip(rows)
                .map(|(snapshot, row)| Row {
                    row,
                    target: Target {
                        profile: profile.clone(),
                        snapshot: snapshot.clone(),
                    },
                    can_delete: snapshot.kind != SnapshotKind::AutoSessionStart,
                })
                .collect()
        })
        .unwrap_or_default();
    view.snapshots.profile = profile;
}
pub(super) fn prepare(
    view: &mut View,
    state: &AppState,
    intent: Intent,
) -> Result<Option<EngineCommand>, String> {
    let command = match intent {
        Intent::Cancel => {
            view.snapshots.restore = None;
            view.workflow += 1;
            None
        }
        Intent::Toggle(profile) => {
            active(&profile, state)?;
            view.snapshots.open = !view.snapshots.open;
            None
        }
        Intent::Create(profile) => {
            active(&profile, state)?;
            Some(command(&profile, SnapshotAction::Manual)?)
        }
        Intent::Restore(target) => {
            current(&target, state)?;
            view.snapshots.restore = Some(target);
            view.workflow += 1;
            None
        }
        Intent::Delete(target) => {
            current(&target, state)?;
            if target.snapshot.kind == SnapshotKind::AutoSessionStart {
                return Err("The session-start recovery snapshot cannot be deleted.".into());
            }
            Some(command(
                &target.profile,
                SnapshotAction::Delete(target.snapshot.clone()),
            )?)
        }
        Intent::Confirm => {
            let target = view
                .snapshots
                .restore
                .as_ref()
                .ok_or("No snapshot restore is awaiting confirmation.")?;
            current(target, state)?;
            let request = command(
                &target.profile,
                SnapshotAction::Restore(target.snapshot.clone()),
            )?;
            view.snapshots.restore = None;
            view.workflow += 1;
            Some(request)
        }
    };
    Ok(command)
}
fn command(profile: &Profile, action: SnapshotAction) -> Result<EngineCommand, String> {
    Ok(EngineCommand::Snapshot(Box::new(SnapshotRequest {
        profile: profile
            .profile
            .clone()
            .ok_or("This action requires the active profile.")?,
        path: profile.path.clone(),
        action,
    })))
}
