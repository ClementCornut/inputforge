//! Captured GUI snapshot requests cannot follow queued loads into another profile.
use super::{make_engine_with_simple_disk_profile, make_profile, simple_modes};
use crate::{
    engine::{EngineCommand, SnapshotAction, SnapshotRequest},
    snapshot::{self, SnapshotKind},
    state::AppState,
};
fn action(state: &AppState, index: usize) -> SnapshotAction {
    match index {
        0 => SnapshotAction::Manual,
        1 => SnapshotAction::Restore(state.active_snapshot_rows[0].clone()),
        _ => SnapshotAction::Delete(state.active_snapshot_rows[0].clone()),
    }
}
fn request(state: &AppState, action: SnapshotAction) -> EngineCommand {
    EngineCommand::Snapshot(Box::new(SnapshotRequest {
        profile: state.active_profile.as_ref().unwrap().id().clone(),
        path: state.profile_path.clone().unwrap(),
        action,
    }))
}
#[test]
fn queued_load_rejects_all_old_profile_snapshot_actions_before_any_side_effect() {
    for index in 0..3 {
        let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
        tx.send(EngineCommand::CreateSnapshot {
            kind: SnapshotKind::Manual,
            label: None,
        })
        .unwrap();
        engine.tick().unwrap();
        let request = {
            let s = state.read();
            request(&s, action(&s, index))
        };
        let other = path.with_file_name("Bravo.toml");
        make_profile(simple_modes(), vec![]).save(&other).unwrap();
        let bytes = std::fs::read(&other).unwrap();
        tx.send(EngineCommand::LoadProfile(other.clone())).unwrap();
        tx.send(request).unwrap();
        engine.tick().unwrap();
        assert_eq!(state.read().profile_path.as_ref(), Some(&other));
        let rows = snapshot::list(&other).unwrap();
        assert_eq!(rows.len(), 1, "old request {index} changed B snapshots");
        assert_eq!(rows[0].kind, SnapshotKind::AutoSessionStart);
        assert_eq!(snapshot::list(&path).unwrap().len(), 1);
        assert_eq!(std::fs::read(&other).unwrap(), bytes);
        assert!(
            state
                .read()
                .warnings
                .iter()
                .any(|w| w.contains("profile changed")),
            "{:?}",
            state.read().warnings
        );
    }
}
#[test]
fn replacement_removal_and_changed_rows_reject_before_recovery_or_delete() {
    for mutation in 0..4 {
        for index in 0..3 {
            let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
            tx.send(EngineCommand::CreateSnapshot {
                kind: SnapshotKind::Manual,
                label: None,
            })
            .unwrap();
            engine.tick().unwrap();
            let request = {
                let s = state.read();
                request(&s, action(&s, index))
            };
            let bytes = std::fs::read(&path).unwrap();
            {
                let mut s = state.write();
                match mutation {
                    0 => s.active_profile = Some(make_profile(simple_modes(), vec![])),
                    1 => s.active_profile = None,
                    2 => {
                        s.active_snapshot_rows.clear();
                    }
                    _ => s.active_snapshot_rows[0].taken_at += chrono::Duration::seconds(1),
                }
            }
            if mutation >= 2 && index == 0 {
                continue;
            }
            tx.send(request).unwrap();
            engine.tick().unwrap();
            assert_eq!(
                snapshot::list(&path).unwrap().len(),
                1,
                "mutation {mutation}, action {index}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert_ne!(state.read().warnings, Vec::<String>::new());
        }
    }
}
#[test]
fn current_guarded_requests_reuse_existing_snapshot_operations() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    tx.send(request(&state.read(), SnapshotAction::Manual))
        .unwrap();
    engine.tick().unwrap();
    let row = state.read().active_snapshot_rows[0].clone();
    tx.send(request(&state.read(), SnapshotAction::Restore(row.clone())))
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(snapshot::list(&path).unwrap().len(), 2);
    tx.send(request(&state.read(), SnapshotAction::Delete(row)))
        .unwrap();
    engine.tick().unwrap();
    let rows = snapshot::list(&path).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, SnapshotKind::AutoBeforeRestore);
}
#[test]
fn failed_queued_load_keeps_snapshot_in_its_captured_active_profile() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let command = request(&state.read(), SnapshotAction::Manual);
    tx.send(EngineCommand::LoadProfile(
        path.with_file_name("missing.toml"),
    ))
    .unwrap();
    tx.send(command).unwrap();
    engine.tick().unwrap();
    assert_eq!(snapshot::list(&path).unwrap().len(), 1);
    assert_eq!(state.read().profile_path.as_ref(), Some(&path));
    assert_ne!(state.read().warnings, Vec::<String>::new());
}
#[test]
fn accepted_guard_preserves_restore_failure_warning_and_live_contents() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    tx.send(request(&state.read(), SnapshotAction::Manual))
        .unwrap();
    engine.tick().unwrap();
    let row = state.read().active_snapshot_rows[0].clone();
    let command = request(&state.read(), SnapshotAction::Restore(row.clone()));
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(
        snapshot::__test_snap_dir(&path)
            .unwrap()
            .join(format!("{}.toml", row.id)),
    )
    .unwrap();
    tx.send(command).unwrap();
    engine.tick().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(
        state
            .read()
            .warnings
            .iter()
            .any(|w| w.to_ascii_lowercase().contains("snapshot"))
    );
    assert!(
        snapshot::list(&path)
            .unwrap()
            .iter()
            .any(|r| r.kind == SnapshotKind::AutoBeforeRestore)
    );
}
