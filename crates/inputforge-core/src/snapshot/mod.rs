//! On-disk profile snapshot store.
//!
//! See `docs/superpowers/specs/2026-04-28-f6-snapshot-preferences-core-design.md`
//! for the full design.

pub use self::config::SnapshotConfig;
pub use self::types::{Snapshot, SnapshotId, SnapshotKind};

pub(crate) mod config;
pub(crate) mod fs;
pub(crate) mod hash;
pub(crate) mod index;
pub(crate) mod types;

use std::path::Path;

use chrono::Utc;
use ulid::Ulid;

use crate::error::{EngineError, Result};

/// Create a snapshot of the profile at `profile_path`.
///
/// `pinned` is derived from `kind`:
/// - `Manual` → `pinned = true` unconditionally.
/// - `AutoSessionStart` / `AutoBeforeRestore` → `pinned = false`.
///
/// Returns `Ok(None)` when `kind == AutoSessionStart`, `cfg.skip_if_unchanged`
/// is true, and the latest existing snapshot has the same `content_hash`.
/// `AutoBeforeRestore` and `Manual` always create.
///
/// Does not call [`prune`] — caller is responsible.
///
/// # Errors
///
/// I/O failure, profile parse failure, or serialization failure.
pub fn create(
    profile_path: &Path,
    kind: SnapshotKind,
    label: Option<String>,
    cfg: &SnapshotConfig,
) -> Result<Option<Snapshot>> {
    // 1. Read live profile bytes.
    let body = std::fs::read_to_string(profile_path)?;
    // 2. Compute canonical content hash (D14).
    let content_hash = hash::hash_canonical_toml(&body)?;
    // 3. Read prior entries ONCE — before any disk write. We must not call
    //    list() after writing the snapshot file: the orphan-recovery path
    //    in list() would pick up the just-written file and return a Vec
    //    that already contains our snapshot, causing a duplicate when we
    //    prepend below.
    let prior = list(profile_path)?;
    // 4. AutoSessionStart dedup: skip if hash matches latest existing entry.
    if matches!(kind, SnapshotKind::AutoSessionStart) && cfg.skip_if_unchanged {
        if let Some(latest) = prior.first() {
            if latest.content_hash == content_hash {
                tracing::info!(
                    target: "snapshot",
                    profile_path = %profile_path.display(),
                    "skipping AutoSessionStart: content unchanged"
                );
                return Ok(None);
            }
        }
    }
    // 5. Build snapshot record.
    let snap = Snapshot {
        id: SnapshotId(Ulid::new()),
        kind,
        label,
        taken_at: Utc::now(),
        content_hash,
        pinned: matches!(kind, SnapshotKind::Manual),
    };
    // 6. Compose snapshot file body: [snapshot_meta] + profile body.
    let snap_dir = fs::snapshots_dir_for(profile_path)?;
    if !snap_dir.exists() {
        std::fs::create_dir_all(&snap_dir).map_err(|source| EngineError::SnapshotDirCreate {
            path: snap_dir.clone(),
            source,
        })?;
    }
    let meta_table = toml::to_string(&MetaWrapper {
        snapshot_meta: snap.clone(),
    })?;
    let combined = format!("{meta_table}\n{body}");
    let snap_path = snap_dir.join(format!("{}.toml", snap.id));
    fs::atomic_write(&snap_path, combined.as_bytes())?;

    // 7. Compose updated index from `prior` + the new entry; do NOT re-call
    //    list() here.
    let mut entries = prior;
    entries.insert(0, snap.clone());
    index::write_index(&snap_dir.join("index.toml"), &entries)?;

    tracing::info!(
        target: "snapshot",
        id = %snap.id,
        kind = ?snap.kind,
        profile_path = %profile_path.display(),
        "snapshot created"
    );
    Ok(Some(snap))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MetaWrapper {
    snapshot_meta: Snapshot,
}

/// List all snapshots for a profile, newest first.
///
/// Reads `<stem>.snapshots/index.toml` and verifies entries against the
/// snapshot files on disk. If the index is missing, unparseable, or
/// out of sync, rebuilds it from snapshot file headers.
///
/// # Errors
///
/// Returns [`crate::error::EngineError::SnapshotDirIo`] on directory
/// read failure.
pub fn list(profile_path: &Path) -> Result<Vec<Snapshot>> {
    let snap_dir = fs::snapshots_dir_for(profile_path)?;
    let index_path = snap_dir.join("index.toml");

    // Try cached path first.
    let cached = index::read_index(&index_path)?;

    // Verify each cached entry's file exists; rebuild if any are missing
    // or if the snapshot dir contains files not represented in the index.
    let needs_rebuild = if cached.is_empty() && snap_dir.exists() {
        true
    } else {
        let mut entries_match = true;
        for entry in &cached {
            let expected = snap_dir.join(format!("{}.toml", entry.id));
            if !expected.exists() {
                entries_match = false;
                break;
            }
        }
        if entries_match {
            // Detect orphan files (present on disk, missing from index).
            count_orphans(&snap_dir, &cached)? > 0
        } else {
            true
        }
    };

    let mut entries = if needs_rebuild {
        let rebuilt = index::rebuild_from_dir(&snap_dir)?;
        // Persist the rebuilt index only when it differs from the cached
        // contents — a redundant write would just rewrite the same bytes.
        // Don't propagate write errors: a failed write is recoverable on
        // the next `list()`.
        if cached != rebuilt {
            if let Err(e) = index::write_index(&index_path, &rebuilt) {
                tracing::warn!(
                    target: "snapshot",
                    path = %index_path.display(),
                    error = %e,
                    "failed to persist rebuilt index"
                );
            }
        }
        rebuilt
    } else {
        cached
    };
    index::ensure_sorted_newest_first(&mut entries);
    Ok(entries)
}

fn count_orphans(snap_dir: &Path, cached: &[Snapshot]) -> Result<usize> {
    use std::collections::HashSet;
    let known: HashSet<String> = cached.iter().map(|s| format!("{}.toml", s.id)).collect();
    let mut orphans = 0usize;
    let read = match std::fs::read_dir(snap_dir) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(source) => {
            return Err(EngineError::SnapshotDirIo {
                path: snap_dir.to_path_buf(),
                source,
            });
        }
    };
    for entry in read.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        // Case-insensitive `.toml` check — paths on Windows are case-insensitive
        // and clippy::case_sensitive_file_extension_comparisons rejects raw
        // `ends_with(".toml")` for that reason.
        let is_toml = Path::new(name_str)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"));
        if !is_toml || name_str == "index.toml" {
            continue;
        }
        if !known.contains(name_str) {
            orphans += 1;
        }
    }
    Ok(orphans)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
