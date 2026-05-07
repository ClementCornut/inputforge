//! Filesystem helpers: layout calculations + atomic write.

use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{EngineError, Result};

/// Compute the snapshots directory for a profile.
///
/// `<profile_dir>/<stem>.snapshots/` where `stem` strips the **first**
/// extension only (`Path::file_stem`). For `TFM_Throttle.toml` the
/// result is `TFM_Throttle.snapshots`.
///
/// # Errors
///
/// Returns [`EngineError::ProfilePathHasNoParent`] when `profile_path`
/// has no parent directory.
pub(crate) fn snapshots_dir_for(profile_path: &Path) -> Result<PathBuf> {
    let parent = profile_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| EngineError::ProfilePathHasNoParent {
            path: profile_path.to_path_buf(),
        })?;
    let stem = profile_path
        .file_stem()
        .ok_or_else(|| EngineError::ProfilePathHasNoParent {
            path: profile_path.to_path_buf(),
        })?;
    let mut dir = parent.join(stem);
    dir.as_mut_os_string().push(".snapshots");
    Ok(dir)
}

/// Compute the snapshots directory for an externally loaded profile.
///
/// External profiles do not live in the `InputForge` profile library; their
/// snapshot history is namespaced by a deterministic SHA-256 hex of the
/// canonical path under `<config_dir>/external_snapshots/<hash>/`. This
/// keeps the snapshot store inside `InputForge`'s config directory rather
/// than next to user-owned files, and reloading the same external path
/// always resolves to the same namespace.
pub(crate) fn external_snapshots_dir_for(canonical_path: &Path) -> PathBuf {
    let path_str = canonical_path.as_os_str().to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    let hash = hex::encode(hasher.finalize());
    crate::settings::AppSettings::config_dir()
        .join("external_snapshots")
        .join(hash)
}

/// Atomically write `bytes` to `dest`.
///
/// Creates the parent directory if needed, writes to a temp file in
/// the same directory as `dest`, then renames into place. Atomic on
/// NTFS and POSIX *only when temp and dest share a volume*; we enforce
/// that by placing the temp file in `dest.parent()`.
///
/// # Errors
///
/// Returns [`EngineError::ProfilePathHasNoParent`] when the destination
/// has no parent directory, [`EngineError::SnapshotDirCreate`] when the
/// parent directory cannot be created, or [`EngineError::Io`] for
/// read/write failures.
pub(crate) fn atomic_write(dest: &Path, bytes: &[u8]) -> Result<()> {
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| EngineError::ProfilePathHasNoParent {
            path: dest.to_path_buf(),
        })?;
    if !parent.exists() {
        std::fs::create_dir_all(parent).map_err(|source| EngineError::SnapshotDirCreate {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.flush()?;
    tmp.persist(dest).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_dir_strips_first_extension_only() {
        let p = PathBuf::from("/data/profiles/TFM_Throttle.toml");
        assert_eq!(
            snapshots_dir_for(&p).unwrap(),
            PathBuf::from("/data/profiles/TFM_Throttle.snapshots")
        );
    }

    #[test]
    fn snapshots_dir_for_path_with_no_parent_errors() {
        let p = PathBuf::from("foo.toml");
        let result = snapshots_dir_for(&p);
        assert!(matches!(
            result,
            Err(EngineError::ProfilePathHasNoParent { .. })
        ));
    }

    #[test]
    fn atomic_write_lands_destination_on_same_volume() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("snap.toml");
        atomic_write(&dest, b"hello").unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"hello");
    }

    #[test]
    fn atomic_write_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("snap.toml");
        std::fs::write(&dest, b"old").unwrap();
        atomic_write(&dest, b"new").unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"new");
    }

    #[test]
    fn atomic_write_leaves_no_temp_file_after_success() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("snap.toml");
        atomic_write(&dest, b"hello").unwrap();
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name())
            .collect();
        // Only `snap.toml` should remain.
        assert_eq!(
            entries.len(),
            1,
            "tempfile must be persisted, not left behind"
        );
        assert_eq!(entries[0].to_str(), Some("snap.toml"));
    }

    #[test]
    fn dropped_temp_does_not_create_destination() {
        // Sanity: tempfile semantics, dropping without persist leaves nothing.
        let dir = tempfile::tempdir().unwrap();
        {
            let _tmp = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
            // Drop without persist.
        }
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .collect();
        assert!(entries.is_empty(), "dropped tempfile must self-clean");
    }
}
