use super::*;
use crate::snapshot::pending_delete::{
    list_visible, pending_manifest_path, purge_expired_pending_deletes, stage_delete,
    undo_delete_by_id,
};
use std::path::PathBuf;

/// Write a minimal valid profile to `profile_path` and return its
/// containing temp dir.
fn fresh_profile_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("TFM_Throttle.toml");
    std::fs::write(
        &path,
        "[profile]\nid = \"550e8400-e29b-41d4-a716-446655440000\"\n\
         name = \"TFM Throttle\"\nstartup_mode = \"Default\"\n\n\
         [modes]\nDefault = []\n",
    )
    .unwrap();
    (dir, path)
}

#[test]
fn create_manual_returns_pinned_snapshot() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let snap = create(&path, SnapshotKind::Manual, None, &cfg)
        .unwrap()
        .unwrap();
    assert_eq!(snap.kind, SnapshotKind::Manual);
    assert!(snap.pinned, "Manual snapshots are auto-pinned");
}

#[test]
fn create_auto_session_start_returns_unpinned() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let snap = create(&path, SnapshotKind::AutoSessionStart, None, &cfg)
        .unwrap()
        .unwrap();
    assert!(!snap.pinned);
}

#[test]
fn create_auto_session_start_dedupes_unchanged_content() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let first = create(&path, SnapshotKind::AutoSessionStart, None, &cfg).unwrap();
    let second = create(&path, SnapshotKind::AutoSessionStart, None, &cfg).unwrap();
    assert!(first.is_some());
    assert!(second.is_none(), "second auto snapshot should dedup");
}

#[test]
fn create_auto_before_restore_never_dedupes() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let a = create(&path, SnapshotKind::AutoBeforeRestore, None, &cfg).unwrap();
    let b = create(&path, SnapshotKind::AutoBeforeRestore, None, &cfg).unwrap();
    assert!(a.is_some() && b.is_some());
}

#[test]
fn snapshot_kind_auto_before_bulk_map_serializes_to_snake_case() {
    #[derive(serde::Serialize)]
    struct Wrapper {
        kind: SnapshotKind,
    }
    let s = toml::to_string(&Wrapper {
        kind: SnapshotKind::AutoBeforeBulkMap,
    })
    .unwrap();
    assert!(s.contains("auto_before_bulk_map"), "got: {s}");
}

#[test]
fn snapshot_kind_auto_before_bulk_map_round_trips_through_toml() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrapper {
        kind: SnapshotKind,
    }
    let s = toml::to_string(&Wrapper {
        kind: SnapshotKind::AutoBeforeBulkMap,
    })
    .unwrap();
    let back: Wrapper = toml::from_str(&s).unwrap();
    assert_eq!(back.kind, SnapshotKind::AutoBeforeBulkMap);
}

#[test]
fn snapshot_kind_auto_before_bulk_map_creates_unpinned_snapshot() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let snap = create(&path, SnapshotKind::AutoBeforeBulkMap, None, &cfg)
        .unwrap()
        .unwrap();
    assert!(!snap.pinned, "AutoBeforeBulkMap is unpinned");
}

#[test]
fn snapshot_kind_auto_before_bulk_map_always_fires_never_deduped() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let a = create(&path, SnapshotKind::AutoBeforeBulkMap, None, &cfg).unwrap();
    let b = create(&path, SnapshotKind::AutoBeforeBulkMap, None, &cfg).unwrap();
    assert!(
        a.is_some() && b.is_some(),
        "AutoBeforeBulkMap must never dedup"
    );
}

#[test]
fn pending_delete_hides_row_until_undo_restores_it() {
    let (dir, profile) = fresh_profile_dir();
    let snapshot = create(
        &profile,
        SnapshotKind::Manual,
        Some("before trim".to_owned()),
        &SnapshotConfig::default(),
    )
    .unwrap()
    .unwrap();
    let pending_dir = dir.path().join("pending");

    let staged = stage_delete(&profile, &snapshot.id, &pending_dir).unwrap();
    assert!(
        list_visible(&profile, &pending_dir)
            .unwrap()
            .iter()
            .all(|row| row.id != snapshot.id)
    );

    undo_delete_by_id(&pending_dir, &snapshot.id).unwrap();
    assert!(
        list_visible(&profile, &pending_dir)
            .unwrap()
            .iter()
            .any(|row| row.id == snapshot.id)
    );
    assert!(!staged.manifest_path.exists());
}

#[test]
fn expired_pending_delete_purges_on_startup_cleanup() {
    let (dir, profile) = fresh_profile_dir();
    let snapshot = create(
        &profile,
        SnapshotKind::Manual,
        Some("delete me".to_owned()),
        &SnapshotConfig::default(),
    )
    .unwrap()
    .unwrap();
    let pending_dir = dir.path().join("pending");

    stage_delete(&profile, &snapshot.id, &pending_dir).unwrap();
    purge_expired_pending_deletes(&pending_dir, chrono::Duration::zero()).unwrap();

    assert!(
        list_visible(&profile, &pending_dir)
            .unwrap()
            .iter()
            .all(|row| row.id != snapshot.id)
    );
    assert!(!pending_manifest_path(&pending_dir, &snapshot.id).exists());
}

#[test]
fn create_skip_dedup_when_skip_if_unchanged_false() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig {
        max_count: 10,
        skip_if_unchanged: false,
    };
    let a = create(&path, SnapshotKind::AutoSessionStart, None, &cfg).unwrap();
    let b = create(&path, SnapshotKind::AutoSessionStart, None, &cfg).unwrap();
    assert!(a.is_some() && b.is_some());
}

// ── list() tests ──────────────────────────────────────────────────────────

#[test]
fn list_empty_when_no_snapshots() {
    let (_dir, path) = fresh_profile_dir();
    assert!(list(&path).unwrap().is_empty());
}

#[test]
fn list_returns_newest_first_by_taken_at() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig {
        max_count: 100,
        skip_if_unchanged: false,
    };
    let a = create(&path, SnapshotKind::Manual, None, &cfg)
        .unwrap()
        .unwrap();
    // Force monotonically increasing wall clock.
    std::thread::sleep(std::time::Duration::from_millis(2));
    std::fs::write(
        &path,
        "[profile]\nid = \"550e8400-e29b-41d4-a716-446655440001\"\n\
        name = \"changed\"\nstartup_mode = \"Default\"\n\n[modes]\nDefault = []\n",
    )
    .unwrap();
    let b = create(&path, SnapshotKind::Manual, None, &cfg)
        .unwrap()
        .unwrap();
    let listed = list(&path).unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, b.id, "newer must come first");
    assert_eq!(listed[1].id, a.id);
}

#[test]
fn list_rebuilds_when_index_missing() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let snap = create(&path, SnapshotKind::Manual, None, &cfg)
        .unwrap()
        .unwrap();

    // Delete index.toml; the snapshot file remains.
    let snap_dir = fs::snapshots_dir_for(&path).unwrap();
    std::fs::remove_file(snap_dir.join("index.toml")).unwrap();

    let listed = list(&path).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, snap.id);
}

#[test]
fn list_skips_files_with_malformed_meta() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let _ = create(&path, SnapshotKind::Manual, None, &cfg)
        .unwrap()
        .unwrap();

    let snap_dir = fs::snapshots_dir_for(&path).unwrap();
    // Drop a garbage TOML file; rebuild must skip it without erroring.
    std::fs::write(snap_dir.join("garbage.toml"), "not [valid] toml = =").unwrap();
    // Force rebuild path.
    std::fs::remove_file(snap_dir.join("index.toml")).unwrap();

    let listed = list(&path).unwrap();
    assert_eq!(listed.len(), 1, "garbage file must be skipped, not error");
}

// ── delete() tests ────────────────────────────────────────────────────────

#[test]
fn delete_removes_file_and_index_entry() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let snap = create(&path, SnapshotKind::Manual, None, &cfg)
        .unwrap()
        .unwrap();
    delete(&path, &snap.id).unwrap();

    let snap_dir = fs::snapshots_dir_for(&path).unwrap();
    assert!(!snap_dir.join(format!("{}.toml", snap.id)).exists());
    assert!(list(&path).unwrap().is_empty());
}

#[test]
fn delete_unknown_id_returns_not_found() {
    let (_dir, path) = fresh_profile_dir();
    let bogus = SnapshotId(Ulid::new());
    let err = delete(&path, &bogus).unwrap_err();
    assert!(matches!(err, EngineError::SnapshotNotFound { .. }));
}

#[test]
fn pin_toggles_persisted_state() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let snap = create(&path, SnapshotKind::AutoSessionStart, None, &cfg)
        .unwrap()
        .unwrap();
    assert!(!snap.pinned);

    pin(&path, &snap.id, true).unwrap();
    assert!(
        list(&path)
            .unwrap()
            .iter()
            .find(|s| s.id == snap.id)
            .unwrap()
            .pinned
    );

    pin(&path, &snap.id, false).unwrap();
    assert!(
        !list(&path)
            .unwrap()
            .iter()
            .find(|s| s.id == snap.id)
            .unwrap()
            .pinned
    );
}

#[test]
fn pin_unknown_id_returns_not_found() {
    let (_dir, path) = fresh_profile_dir();
    let err = pin(&path, &SnapshotId(Ulid::new()), true).unwrap_err();
    assert!(matches!(err, EngineError::SnapshotNotFound { .. }));
}

#[test]
fn rename_updates_label() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let snap = create(&path, SnapshotKind::Manual, None, &cfg)
        .unwrap()
        .unwrap();

    rename(&path, &snap.id, Some("new label".to_owned())).unwrap();
    let listed = list(&path).unwrap();
    assert_eq!(listed[0].label.as_deref(), Some("new label"));

    rename(&path, &snap.id, None).unwrap();
    assert!(list(&path).unwrap()[0].label.is_none());
}

// ── restore() tests ───────────────────────────────────────────────────────

#[test]
fn restore_strips_meta_and_writes_profile() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let original_body = std::fs::read_to_string(&path).unwrap();
    let snap = create(&path, SnapshotKind::Manual, None, &cfg)
        .unwrap()
        .unwrap();

    // Mutate the live profile.
    std::fs::write(
        &path,
        "[profile]\nid = \"550e8400-e29b-41d4-a716-446655440099\"\n\
        name = \"changed\"\nstartup_mode = \"Default\"\n\n[modes]\nDefault = []\n",
    )
    .unwrap();

    // Restore. Snapshot file is unchanged on disk; live profile is rewritten.
    restore(&path, &snap.id).unwrap();

    let restored = std::fs::read_to_string(&path).unwrap();
    // Restored body must NOT contain the meta table.
    assert!(!restored.contains("[snapshot_meta]"));
    // Round-trip equality (TOML reformats; semantic equality only).
    let original_value: toml::Value = toml::from_str(&original_body).unwrap();
    let restored_value: toml::Value = toml::from_str(&restored).unwrap();
    assert_eq!(original_value, restored_value);
}

#[test]
fn restore_unknown_id_returns_not_found() {
    let (_dir, path) = fresh_profile_dir();
    let err = restore(&path, &SnapshotId(Ulid::new())).unwrap_err();
    assert!(matches!(err, EngineError::SnapshotNotFound { .. }));
}

#[test]
fn restore_errors_when_meta_table_missing() {
    let (_dir, path) = fresh_profile_dir();
    let snap_dir = fs::snapshots_dir_for(&path).unwrap();
    std::fs::create_dir_all(&snap_dir).unwrap();
    let id = SnapshotId(Ulid::new());
    let snap_path = snap_dir.join(format!("{id}.toml"));
    // Valid TOML but no [snapshot_meta] table.
    std::fs::write(
        &snap_path,
        "[profile]\nid = \"550e8400-e29b-41d4-a716-446655440000\"\n\
         name = \"meta-less\"\nstartup_mode = \"Default\"\n\n[modes]\nDefault = []\n",
    )
    .unwrap();

    let err = restore(&path, &id).unwrap_err();
    match err {
        EngineError::SnapshotCorrupt { reason, .. } => {
            assert!(
                reason.contains("missing [snapshot_meta]"),
                "expected missing-meta reason, got: {reason}",
            );
        }
        other => panic!("expected SnapshotCorrupt, got {other:?}"),
    }
}

// ── prune() tests ─────────────────────────────────────────────────────────

#[test]
fn prune_evicts_oldest_unpinned() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig {
        max_count: 2,
        skip_if_unchanged: false,
    };

    // Create 3 unpinned snapshots, mutating profile content between each
    // so dedup wouldn't apply even if the kind allowed it.
    let mut ids = Vec::new();
    for i in 0..3 {
        std::fs::write(
            &path,
            format!(
                "[profile]\nid = \"550e8400-e29b-41d4-a716-44665544000{i}\"\n\
            name = \"v{i}\"\nstartup_mode = \"Default\"\n\n[modes]\nDefault = []\n"
            ),
        )
        .unwrap();
        let s = create(&path, SnapshotKind::AutoSessionStart, None, &cfg)
            .unwrap()
            .unwrap();
        ids.push(s.id);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    let evicted = prune(&path, &cfg).unwrap();
    assert_eq!(evicted, 1);
    let remaining: Vec<_> = list(&path).unwrap().iter().map(|s| s.id).collect();
    assert!(remaining.contains(&ids[1]));
    assert!(remaining.contains(&ids[2]));
    assert!(!remaining.contains(&ids[0]), "oldest must be evicted");
}

#[test]
fn prune_skips_pinned_snapshots() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig {
        max_count: 1,
        skip_if_unchanged: false,
    };

    let s1 = create(&path, SnapshotKind::AutoSessionStart, None, &cfg)
        .unwrap()
        .unwrap();
    pin(&path, &s1.id, true).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));

    std::fs::write(
        &path,
        "[profile]\nid = \"550e8400-e29b-41d4-a716-446655440042\"\n\
        name = \"v2\"\nstartup_mode = \"Default\"\n\n[modes]\nDefault = []\n",
    )
    .unwrap();
    let s2 = create(&path, SnapshotKind::AutoSessionStart, None, &cfg)
        .unwrap()
        .unwrap();

    let _ = prune(&path, &cfg).unwrap();
    let remaining: Vec<_> = list(&path).unwrap().iter().map(|s| s.id).collect();
    assert!(remaining.contains(&s1.id), "pinned must survive");
    assert!(remaining.contains(&s2.id));
}

#[test]
fn prune_no_op_under_max_count() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig {
        max_count: 10,
        skip_if_unchanged: false,
    };
    create(&path, SnapshotKind::Manual, None, &cfg).unwrap();
    assert_eq!(prune(&path, &cfg).unwrap(), 0);
}
