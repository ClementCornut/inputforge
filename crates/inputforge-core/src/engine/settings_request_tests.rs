//! Guarded settings deltas cannot prune a replacement profile or lose queued fields.
use super::{make_engine_with_simple_disk_profile, make_profile, simple_modes};
use crate::engine::{EngineCommand, PruneGuard, SettingsChange, SettingsRequest};
fn send(change: SettingsChange, serial: u64) -> EngineCommand {
    EngineCommand::Settings(Box::new(SettingsRequest {
        owner: 5,
        serial,
        change,
    }))
}
#[test]
fn queued_profile_load_cancels_buffer_change_before_save_or_prune() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let guard = PruneGuard::capture(&state.read());
    let other = path.with_file_name("Other.toml");
    make_profile(simple_modes(), vec![]).save(&other).unwrap();
    let original = state.read().snapshot_config.clone();
    tx.send(EngineCommand::LoadProfile(other.clone())).unwrap();
    tx.send(send(SettingsChange::Max { value: 1, guard }, 1))
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(state.read().profile_path.as_ref(), Some(&other));
    assert_eq!(state.read().snapshot_config, original);
    assert_eq!(state.read().settings_ack, Some((5, 1)));
    assert!(state.read().warnings.iter().any(|w| w.contains("changed")));
}
#[test]
fn queued_skip_and_buffer_deltas_preserve_each_other_and_mapping_ack() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_simple_disk_profile();
    let mut guard = PruneGuard::capture(&state.read());
    guard.config.skip_if_unchanged = false;
    state.write().mapping_edit_barrier = Some((7, 8));
    tx.send(send(SettingsChange::Skip(false), 1)).unwrap();
    tx.send(send(SettingsChange::Max { value: 25, guard }, 2))
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(state.read().snapshot_config.max_count, 25);
    assert!(!state.read().snapshot_config.skip_if_unchanged);
    assert_eq!(state.read().settings_ack, Some((5, 2)));
    assert_eq!(state.read().mapping_edit_barrier, Some((7, 8)));
}
#[test]
fn changed_count_or_config_and_invalid_bounds_do_not_save() {
    for mutation in 0..4 {
        let (mut engine, state, tx, _dir, _path) = make_engine_with_simple_disk_profile();
        let mut guard = PruneGuard::capture(&state.read());
        let value = match mutation {
            0 => {
                guard.unpinned += 1;
                1
            }
            1 => {
                guard.config.skip_if_unchanged = false;
                1
            }
            2 => 0,
            _ => 101,
        };
        let original = state.read().snapshot_config.clone();
        tx.send(send(SettingsChange::Max { value, guard }, 1))
            .unwrap();
        engine.tick().unwrap();
        assert_eq!(state.read().snapshot_config, original);
        assert_eq!(state.read().settings_ack, Some((5, 1)));
        assert_ne!(state.read().warnings, Vec::<String>::new());
    }
}
#[test]
fn failed_save_acknowledges_without_changing_config_or_pruning() {
    let mut h = super::EngineHarness::new();
    h.create_and_load_profile("Alpha").unwrap();
    let guard = PruneGuard::capture(&h.state());
    let original = h.engine.settings.snapshot.clone();
    h.force_settings_path_to_unwritable();
    h.dispatch(send(SettingsChange::Max { value: 1, guard }, 9))
        .unwrap();
    assert_eq!(h.state().settings_ack, Some((5, 9)));
    assert_eq!(h.state().snapshot_config, original);
    assert!(
        h.state()
            .warnings
            .iter()
            .any(|w| w.contains("Could not save settings"))
    );
}
#[test]
fn prune_failure_after_save_acknowledges_and_warns_without_claiming_removal() {
    let mut h = super::EngineHarness::new();
    h.create_and_load_profile("Alpha").unwrap();
    h.dispatch(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::AutoBeforeRestore,
        label: None,
    })
    .unwrap();
    let namespace = crate::snapshot::pending_delete::resolve_snapshot_namespace(
        &h.state(),
        h.engine.config_dir(),
    )
    .unwrap();
    let guard = PruneGuard::capture(&h.state());
    std::fs::remove_dir_all(&namespace).unwrap();
    std::fs::write(&namespace, b"blocked namespace").unwrap();
    h.dispatch(send(SettingsChange::Max { value: 1, guard }, 10))
        .unwrap();
    assert_eq!(h.state().settings_ack, Some((5, 10)));
    assert_eq!(h.state().snapshot_config.max_count, 1);
    assert!(
        h.state()
            .warnings
            .iter()
            .any(|w| w.contains("Snapshot prune failed after settings save"))
    );
    assert!(
        h.state()
            .warnings
            .iter()
            .all(|w| !w.contains("removed from"))
    );
}
#[test]
fn pruning_keeps_pinned_snapshots_and_only_removes_excess_unpinned_rows() {
    let mut h = super::EngineHarness::new();
    h.create_and_load_profile("Alpha").unwrap();
    h.dispatch(EngineCommand::CreateSnapshot {
        kind: crate::snapshot::SnapshotKind::Manual,
        label: None,
    })
    .unwrap();
    for _ in 0..4 {
        h.dispatch(EngineCommand::CreateSnapshot {
            kind: crate::snapshot::SnapshotKind::AutoBeforeRestore,
            label: None,
        })
        .unwrap();
    }
    let pinned = h
        .state()
        .active_snapshot_rows
        .iter()
        .filter(|r| r.pinned)
        .count();
    assert!(pinned > 0);
    let guard = PruneGuard::capture(&h.state());
    h.dispatch(send(SettingsChange::Max { value: 2, guard }, 11))
        .unwrap();
    assert_eq!(
        h.state()
            .active_snapshot_rows
            .iter()
            .filter(|r| r.pinned)
            .count(),
        pinned
    );
    assert_eq!(
        h.state()
            .active_snapshot_rows
            .iter()
            .filter(|r| !r.pinned)
            .count(),
        2
    );
}
