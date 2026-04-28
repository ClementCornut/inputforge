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

/// Return all snapshots for `profile_path`, ordered newest-first.
///
/// (Stub — full implementation lands once `list` is complete.)
///
/// # Errors
///
/// Currently infallible; reserved for I/O errors once the real
/// implementation replaces this stub.
pub fn list(_profile_path: &Path) -> Result<Vec<Snapshot>> {
    Ok(Vec::new())
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
    #[ignore = "dedup logic depends on list(); enable once list() lands"]
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
}
