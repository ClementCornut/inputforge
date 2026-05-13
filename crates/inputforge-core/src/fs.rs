// Rust guideline compliant 2026-05-13

use std::io::Write;
use std::path::Path;

use crate::error::{EngineError, Result};

pub(crate) fn atomic_write(dest: &Path, bytes: &[u8]) -> Result<()> {
    atomic_write_with(dest, bytes, |tmp, dest| {
        tmp.persist(dest).map(|_| ()).map_err(|err| err.error)
    })
}

fn atomic_write_with(
    dest: &Path,
    bytes: &[u8],
    persist: impl FnOnce(tempfile::NamedTempFile, &Path) -> std::io::Result<()>,
) -> Result<()> {
    let parent = dest
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
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
    persist(tmp, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_failure_keeps_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("sidecar.toml");
        std::fs::write(&dest, "stable").unwrap();

        let err = atomic_write_with(&dest, b"new", |tmp, dest| {
            assert_eq!(std::fs::read(tmp.path()).unwrap(), b"new");
            assert_eq!(std::fs::read_to_string(dest).unwrap(), "stable");

            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "injected persist failure",
            ))
        })
        .unwrap_err();

        assert!(matches!(err, EngineError::Io(_)));
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "stable");
    }
}
