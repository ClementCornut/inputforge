//! Snapshot decisions run through real app bindings and current engine state.
#[path = "snapshots/document.rs"]
mod document;
#[path = "snapshots/fixture.rs"]
mod fixture;
#[path = "snapshots/shortcut.rs"]
mod shortcut;
use dioxus::prelude::*;
use inputforge_core::{
    engine::{EngineCommand, SnapshotAction, SnapshotRequest},
    snapshot::SnapshotKind,
};
use inputforge_gui_dx::profiles::snapshots::Intent;
#[test]
fn manual_and_delete_validate_current_profile_and_protected_rows() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        let target = f.view.peek().snapshots.profile.clone().unwrap();
        assert!(f.send(Intent::Create(target.clone())));
        assert_eq!(
            f.rx.try_recv().unwrap(),
            EngineCommand::Snapshot(Box::new(SnapshotRequest {
                profile: target.profile.clone().unwrap(),
                path: target.path.clone(),
                action: SnapshotAction::Manual
            }))
        );
        let row = f.view.peek().snapshots.rows[0].target.clone();
        f.state.write().active_snapshot_rows[0].kind = SnapshotKind::AutoSessionStart;
        f.c.refresh(f.owner);
        assert!(!f.view.peek().snapshots.rows[0].can_delete);
        assert!(!f.send(Intent::Delete(row)));
        let protected = f.view.peek().snapshots.rows[0].target.clone();
        assert!(!f.send(Intent::Delete(protected)));
        f.state.write().active_profile = None;
        assert!(!f.send(Intent::Create(target)));
        assert!(f.view.peek().error.is_some());
        assert_eq!(
            f.rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    });
}
#[test]
fn restore_requires_confirmation_and_cancel_never_delivers() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        let row = f.view.peek().snapshots.rows[0].target.clone();
        assert!(f.send(Intent::Restore(row.clone())));
        assert!(f.view.peek().snapshots.restore.is_some());
        assert_eq!(
            f.rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
        assert!(f.send(Intent::Cancel));
        assert!(f.view.peek().snapshots.restore.is_none());
        assert!(!f.send(Intent::Confirm));
        assert!(f.send(Intent::Restore(row.clone())));
        assert!(f.send(Intent::Confirm));
        assert_eq!(
            f.rx.try_recv().unwrap(),
            EngineCommand::Snapshot(Box::new(SnapshotRequest {
                profile: row.profile.profile.clone().unwrap(),
                path: row.profile.path.clone(),
                action: SnapshotAction::Restore(row.snapshot.clone())
            }))
        );
        assert!(f.view.peek().snapshots.restore.is_none());
        assert!(f.view.peek().error.is_none());
    });
}
#[test]
fn failed_delivery_keeps_restore_confirmation_and_drawer_state() {
    let (dom, f) = fixture::fixture(false);
    dom.in_scope(ScopeId::ROOT, || {
        let row = f.view.peek().snapshots.rows[0].target.clone();
        assert!(f.send(Intent::Restore(row)));
        assert!(!f.send(Intent::Confirm));
        assert!(f.view.peek().snapshots.restore.is_some());
        assert!(f.view.peek().snapshots.open);
        assert!(f.view.peek().error.is_some());
    });
}
#[test]
fn retained_confirmation_rejects_missing_replaced_snapshot_or_same_name_profile() {
    for mutation in 0..3 {
        let (dom, f) = fixture::fixture(true);
        dom.in_scope(ScopeId::ROOT, || {
            let row = f.view.peek().snapshots.rows[0].target.clone();
            assert!(f.send(Intent::Restore(row)));
            match mutation {
                0 => f.state.write().active_snapshot_rows.clear(),
                1 => {
                    f.state.write().active_snapshot_rows[0].taken_at +=
                        chrono::Duration::seconds(1);
                }
                _ => f.state.write().active_profile = Some(fixture::profile()),
            }
            assert!(!f.send(Intent::Confirm));
            assert!(f.view.peek().snapshots.restore.is_some());
            assert!(f.view.peek().error.is_some());
            assert_eq!(
                f.rx.try_recv().unwrap_err(),
                std::sync::mpsc::TryRecvError::Empty
            );
            assert!(f.send(Intent::Cancel));
        });
    }
}
#[test]
fn drawer_transitions_and_old_mounted_callbacks_are_app_owned() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        let profile = f.view.peek().snapshots.profile.clone().unwrap();
        assert!(f.send(Intent::Toggle(profile.clone())));
        assert!(!f.view.peek().snapshots.open);
        assert!(f.send(Intent::Toggle(profile.clone())));
        f.bindings.unmount.call(f.owner);
        assert!(!f.send(Intent::Create(profile)));
        assert_eq!(
            f.rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    });
}
#[test]
fn delete_delivers_once_and_stale_row_or_workflow_cannot_target_current_snapshot() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        let row = f.view.peek().snapshots.rows[0].target.clone();
        assert!(f.send(Intent::Delete(row.clone())));
        assert_eq!(
            f.rx.try_recv().unwrap(),
            EngineCommand::Snapshot(Box::new(SnapshotRequest {
                profile: row.profile.profile.clone().unwrap(),
                path: row.profile.path.clone(),
                action: SnapshotAction::Delete(row.snapshot.clone())
            }))
        );
        let old_workflow = f.view.peek().workflow;
        assert!(f.send(Intent::Restore(row.clone())));
        assert!(f.send(Intent::Cancel));
        assert!(!f.bindings.dispatch.call((
            f.owner,
            old_workflow,
            inputforge_gui_dx::profiles::Intent::Snapshot(Intent::Restore(row.clone()))
        )));
        f.state.write().active_snapshot_rows.clear();
        assert!(!f.send(Intent::Delete(row)));
        assert_eq!(
            f.rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    });
}
#[test]
fn active_row_snapshot_availability_uses_current_profile_instead_of_old_library_frame() {
    let (dom, f) = fixture::fixture(true);
    dom.in_scope(ScopeId::ROOT, || {
        for (name, is_active) in [("Alpha", false), ("Bravo", true)] {
            f.state
                .write()
                .profile_library_rows
                .push(inputforge_core::state::ProfileLibraryRow {
                    name: name.into(),
                    path: format!("/profiles/{name}.toml").into(),
                    profile_id: None,
                    origin: inputforge_core::state::ProfileOrigin::Library,
                    is_active,
                    mode_count: 1,
                    last_edited_at: None,
                });
        }
        f.c.refresh(f.owner);
        let view = f.view.peek();
        for row in &view.rows {
            assert_eq!(
                row.row.can_snapshot_now, row.row.is_active,
                "{}",
                row.row.name
            );
        }
    });
}
#[test]
fn drawer_actions_keep_captured_identity_behind_queued_profile_load() {
    for index in 0..3 {
        let (dom, f) = fixture::fixture(true);
        dom.in_scope(ScopeId::ROOT, || {
            let profile = f.view.peek().snapshots.profile.clone().unwrap();
            let row = f.view.peek().snapshots.rows[0].target.clone();
            f.state
                .write()
                .profile_library_rows
                .push(inputforge_core::state::ProfileLibraryRow {
                    name: "Bravo".into(),
                    path: "/profiles/Bravo.toml".into(),
                    profile_id: None,
                    origin: inputforge_core::state::ProfileOrigin::Library,
                    is_active: false,
                    mode_count: 1,
                    last_edited_at: None,
                });
            f.c.refresh(f.owner);
            let next = f
                .view
                .peek()
                .rows
                .iter()
                .find(|r| r.target.name == "Bravo")
                .unwrap()
                .target
                .clone();
            assert!(f.c.dispatch(f.owner, inputforge_gui_dx::profiles::Intent::Open(next)));
            match index {
                0 => {
                    assert!(f.send(Intent::Create(profile)));
                }
                1 => {
                    assert!(f.send(Intent::Restore(row)));
                    assert!(f.send(Intent::Confirm));
                }
                _ => {
                    assert!(f.send(Intent::Delete(row)));
                }
            }
            assert_eq!(
                f.rx.try_recv().unwrap(),
                EngineCommand::LoadProfile("/profiles/Bravo.toml".into())
            );
            let EngineCommand::Snapshot(request) = f.rx.try_recv().unwrap() else {
                panic!("snapshot must carry an execution-time profile guard");
            };
            assert_eq!(
                request.profile,
                *f.state.read().active_profile.as_ref().unwrap().id()
            );
            assert_eq!(
                request.path,
                std::path::PathBuf::from("/profiles/Alpha.toml")
            );
        });
    }
}
