//! Filesystem helpers: layout calculations + atomic write.

use std::io::Write;
use std::path::{Path, PathBuf};

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
#[allow(dead_code, reason = "wired in Task 7")]
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
#[allow(dead_code, reason = "wired in Task 7")]
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
}
