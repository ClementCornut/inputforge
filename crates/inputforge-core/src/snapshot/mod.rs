//! On-disk profile snapshot store.
//!
//! See `docs/superpowers/specs/2026-04-28-f6-snapshot-preferences-core-design.md`
//! for the full design.

pub use self::config::SnapshotConfig;
pub use self::types::{Snapshot, SnapshotId, SnapshotKind};

#[cfg(test)]
#[allow(
    unused_imports,
    reason = "re-exported for engine/tests.rs; clippy sees it as unused in the lib target"
)]
pub(crate) use self::fs::snapshots_dir_for as __test_snap_dir;

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
    tracing::debug!(
        target: "snapshot",
        profile_path = %profile_path.display(),
        count = entries.len(),
        rebuilt = needs_rebuild,
        "snapshot list returned"
    );
    Ok(entries)
}

/// Delete a snapshot by id.
///
/// # Errors
///
/// Returns [`EngineError::SnapshotNotFound`] if no snapshot with `id`
/// exists, or [`EngineError::Io`] on filesystem failure.
pub fn delete(profile_path: &Path, id: &SnapshotId) -> Result<()> {
    let snap_dir = fs::snapshots_dir_for(profile_path)?;
    let snap_path = snap_dir.join(format!("{id}.toml"));
    if !snap_path.exists() {
        return Err(EngineError::SnapshotNotFound { id: id.to_string() });
    }

    // Read the index BEFORE the destructive op so a transient list() failure
    // leaves the snapshot file intact and the caller's Err matches reality.
    let mut entries = list(profile_path)?;
    std::fs::remove_file(&snap_path)?;
    entries.retain(|s| s.id != *id);
    index::write_index(&snap_dir.join("index.toml"), &entries)?;

    tracing::info!(
        target: "snapshot",
        id = %id,
        profile_path = %profile_path.display(),
        "snapshot deleted"
    );
    Ok(())
}

/// Pin or unpin a snapshot. Pinned snapshots are exempt from FIFO eviction.
///
/// # Errors
///
/// Returns [`EngineError::SnapshotNotFound`] when `id` is unknown,
/// [`EngineError::Io`] on filesystem failure.
pub fn pin(profile_path: &Path, id: &SnapshotId, pinned: bool) -> Result<()> {
    mutate_meta(profile_path, id, |snap| snap.pinned = pinned)?;
    tracing::info!(
        target: "snapshot",
        id = %id,
        pinned,
        "snapshot pin updated"
    );
    Ok(())
}

/// Rename a snapshot's display label. Pass `None` to clear.
///
/// # Errors
///
/// Returns [`EngineError::SnapshotNotFound`] when `id` is unknown,
/// [`EngineError::Io`] on filesystem failure.
pub fn rename(profile_path: &Path, id: &SnapshotId, label: Option<String>) -> Result<()> {
    let log_label = label.clone();
    mutate_meta(profile_path, id, |snap| snap.label = label)?;
    tracing::info!(
        target: "snapshot",
        id = %id,
        ?log_label,
        "snapshot renamed"
    );
    Ok(())
}

fn mutate_meta(profile_path: &Path, id: &SnapshotId, f: impl FnOnce(&mut Snapshot)) -> Result<()> {
    let snap_dir = fs::snapshots_dir_for(profile_path)?;
    let snap_path = snap_dir.join(format!("{id}.toml"));
    if !snap_path.exists() {
        return Err(EngineError::SnapshotNotFound { id: id.to_string() });
    }
    let body = std::fs::read_to_string(&snap_path)?;
    // Parse the full file as a Value, mutate the meta sub-table, re-serialize.
    let mut value: toml::Value =
        toml::from_str(&body).map_err(|e| EngineError::SnapshotCorrupt {
            path: snap_path.clone(),
            reason: e.to_string(),
        })?;
    let meta_table = value
        .as_table_mut()
        .and_then(|t| t.remove("snapshot_meta"))
        .ok_or_else(|| EngineError::SnapshotCorrupt {
            path: snap_path.clone(),
            reason: "missing [snapshot_meta] table".to_owned(),
        })?;
    let mut snap: Snapshot =
        meta_table
            .try_into()
            .map_err(|e: toml::de::Error| EngineError::SnapshotCorrupt {
                path: snap_path.clone(),
                reason: e.to_string(),
            })?;
    f(&mut snap);

    // Re-serialize: meta wrapper first, then the rest of the value.
    let meta_str = toml::to_string(&MetaWrapper {
        snapshot_meta: snap.clone(),
    })?;
    let rest_str = toml::to_string(&value)?;
    let combined = format!("{meta_str}\n{rest_str}");

    // Read the index BEFORE the destructive op so a transient list() failure
    // leaves the snapshot file's meta untouched.
    let mut entries = list(profile_path)?;
    fs::atomic_write(&snap_path, combined.as_bytes())?;
    if let Some(slot) = entries.iter_mut().find(|s| s.id == *id) {
        *slot = snap;
    }
    index::write_index(&snap_dir.join("index.toml"), &entries)?;
    Ok(())
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

/// Restore the live profile to a snapshot's content.
///
/// Strips `[snapshot_meta]` from the snapshot file and atomically
/// writes the result to `profile_path`. Caller (engine) is responsible
/// for taking `AutoBeforeRestore` first and reloading in-memory state
/// after this returns.
///
/// # Errors
///
/// [`EngineError::SnapshotNotFound`] when `id` is unknown,
/// [`EngineError::SnapshotCorrupt`] when the snapshot file lacks a
/// parseable `[snapshot_meta]` header, or [`EngineError::Io`] on
/// filesystem failure.
pub fn restore(profile_path: &Path, id: &SnapshotId) -> Result<()> {
    let snap_dir = fs::snapshots_dir_for(profile_path)?;
    let snap_path = snap_dir.join(format!("{id}.toml"));
    if !snap_path.exists() {
        return Err(EngineError::SnapshotNotFound { id: id.to_string() });
    }
    let body = std::fs::read_to_string(&snap_path)?;
    let mut value: toml::Value =
        toml::from_str(&body).map_err(|e| EngineError::SnapshotCorrupt {
            path: snap_path.clone(),
            reason: e.to_string(),
        })?;
    let table = value
        .as_table_mut()
        .ok_or_else(|| EngineError::SnapshotCorrupt {
            path: snap_path.clone(),
            reason: "snapshot file is not a TOML table".into(),
        })?;
    if table.remove("snapshot_meta").is_none() {
        return Err(EngineError::SnapshotCorrupt {
            path: snap_path.clone(),
            reason: "missing [snapshot_meta] header".into(),
        });
    }
    let stripped = toml::to_string(&value)?;
    fs::atomic_write(profile_path, stripped.as_bytes())?;

    tracing::info!(
        target: "snapshot",
        id = %id,
        profile_path = %profile_path.display(),
        "snapshot restored to live profile"
    );
    Ok(())
}

/// Apply FIFO eviction down to `cfg.max_count`, skipping pinned
/// snapshots. Returns the number of snapshots evicted.
///
/// # Errors
///
/// Returns [`EngineError::SnapshotDirIo`] on directory read failure,
/// or [`EngineError::Io`] on file delete failure.
pub fn prune(profile_path: &Path, cfg: &SnapshotConfig) -> Result<usize> {
    let entries = list(profile_path)?;
    let unpinned_count = entries.iter().filter(|s| !s.pinned).count();
    if unpinned_count <= cfg.max_count {
        return Ok(0);
    }
    // `entries` is newest-first; evict the oldest unpinned first.
    let to_evict = unpinned_count - cfg.max_count;
    let mut victims: Vec<SnapshotId> = entries
        .iter()
        .rev() // oldest first
        .filter(|s| !s.pinned)
        .take(to_evict)
        .map(|s| s.id)
        .collect();

    let mut evicted = 0usize;
    while let Some(id) = victims.pop() {
        match delete(profile_path, &id) {
            Ok(()) => evicted += 1,
            Err(e) => {
                tracing::warn!(
                    target: "snapshot",
                    id = %id,
                    error = %e,
                    "prune: delete failed; continuing"
                );
            }
        }
    }
    if evicted > 0 {
        tracing::info!(
            target: "snapshot",
            evicted,
            "snapshots pruned"
        );
    } else {
        tracing::debug!(
            target: "snapshot",
            profile_path = %profile_path.display(),
            "prune: nothing to evict"
        );
    }
    Ok(evicted)
}

#[cfg(test)]
mod tests;
