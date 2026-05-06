//! Stages snapshot deletes so they can be undone.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};

use crate::error::Result;

use super::fs::snapshots_dir_for;
use super::index::{read_index, write_index};
use super::{PendingSnapshotDelete, Snapshot, SnapshotId};

/// Return the pending-delete manifest path for `id`.
#[must_use]
pub fn pending_manifest_path(pending_dir: &Path, id: &SnapshotId) -> PathBuf {
    pending_dir.join(format!("{id}.pending.toml"))
}

/// Stage a snapshot delete by moving the file and writing a manifest.
///
/// # Errors
///
/// Returns an error when the snapshot file cannot be found, the pending
/// directory cannot be created, metadata cannot be serialized, or file moves
/// and index rewrites fail.
pub fn stage_delete(
    profile_path: &Path,
    id: &SnapshotId,
    pending_dir: &Path,
) -> Result<PendingSnapshotDelete> {
    std::fs::create_dir_all(pending_dir)?;
    let snapshots_dir = snapshots_dir_for(profile_path)?;
    let original_path = snapshots_dir.join(format!("{id}.toml"));
    let staged_path = pending_dir.join(format!("{id}.toml"));
    let manifest_path = pending_manifest_path(pending_dir, id);

    std::fs::metadata(&original_path)?;
    let staged = PendingSnapshotDelete {
        id: *id,
        profile_path: profile_path.to_path_buf(),
        original_path: original_path.clone(),
        staged_path: staged_path.clone(),
        manifest_path: manifest_path.clone(),
        deleted_at: Utc::now(),
    };
    let body = toml::to_string_pretty(&staged)?;
    std::fs::write(&manifest_path, body)?;
    std::fs::rename(&original_path, &staged_path)?;

    let index_path = snapshots_dir.join("index.toml");
    let mut entries = read_index(&index_path)?;
    entries.retain(|snapshot| snapshot.id != *id);
    write_index(&index_path, &entries)?;

    Ok(staged)
}

/// Undo a staged snapshot delete by id.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read, the original directory
/// cannot be recreated, the staged file cannot be moved back, or the manifest
/// cannot be removed.
pub fn undo_delete_by_id(pending_dir: &Path, id: &SnapshotId) -> Result<()> {
    let manifest_path = pending_manifest_path(pending_dir, id);
    let manifest = read_manifest(&manifest_path)?;
    if let Some(parent) = manifest.original_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&manifest.staged_path, &manifest.original_path)?;
    std::fs::remove_file(&manifest_path)?;
    Ok(())
}

/// Purge pending deletes older than `max_age`.
///
/// # Errors
///
/// Returns an error when pending manifests cannot be read or removed.
pub fn purge_expired_pending_deletes(pending_dir: &Path, max_age: Duration) -> Result<()> {
    if !pending_dir.exists() {
        return Ok(());
    }
    let cutoff = Utc::now() - max_age;
    for entry in std::fs::read_dir(pending_dir)? {
        let path = entry?.path();
        if !is_pending_manifest(&path) {
            continue;
        }
        let manifest = read_manifest(&path)?;
        if manifest.deleted_at <= cutoff {
            match std::fs::remove_file(&manifest.staged_path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// List snapshots excluding rows with pending-delete manifests.
///
/// # Errors
///
/// Returns errors from the underlying snapshot list or pending-manifest reads.
pub fn list_visible(profile_path: &Path, pending_dir: &Path) -> Result<Vec<Snapshot>> {
    let pending = pending_ids_for_profile(profile_path, pending_dir)?;
    Ok(super::list(profile_path)?
        .into_iter()
        .filter(|snapshot| !pending.contains(&snapshot.id))
        .collect())
}

fn pending_ids_for_profile(profile_path: &Path, pending_dir: &Path) -> Result<HashSet<SnapshotId>> {
    if !pending_dir.exists() {
        return Ok(HashSet::new());
    }
    let mut ids = HashSet::new();
    for entry in std::fs::read_dir(pending_dir)? {
        let path = entry?.path();
        if !is_pending_manifest(&path) {
            continue;
        }
        let manifest = read_manifest(&path)?;
        if manifest.profile_path == profile_path {
            ids.insert(manifest.id);
        }
    }
    Ok(ids)
}

fn read_manifest(path: &Path) -> Result<PendingSnapshotDelete> {
    let body = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&body)?)
}

fn is_pending_manifest(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".pending.toml"))
}
