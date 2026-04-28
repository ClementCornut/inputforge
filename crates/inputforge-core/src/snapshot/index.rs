//! `index.toml` cache: read, write, rebuild from snapshot file headers.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::fs::atomic_write;
use super::types::Snapshot;

#[allow(dead_code, reason = "callers wired in later")]
#[derive(Debug, Serialize, Deserialize, Default)]
struct IndexFile {
    #[serde(default)]
    entries: Vec<Snapshot>,
}

/// Read the index file at `path`. Returns an empty vec if the file is
/// missing, unparseable, or truncated — these conditions are recoverable
/// by a rebuild from snapshot file headers, performed by the caller.
#[allow(dead_code, reason = "callers wired in later")]
#[allow(
    clippy::unnecessary_wraps,
    reason = "Result<Vec<>> signals operation completion"
)]
pub(crate) fn read_index(path: &Path) -> Result<Vec<Snapshot>> {
    match std::fs::read_to_string(path) {
        Ok(s) => match toml::from_str::<IndexFile>(&s) {
            Ok(f) => Ok(f.entries),
            Err(e) => {
                tracing::warn!(
                    target: "snapshot",
                    path = %path.display(),
                    error = %e,
                    "snapshot index unparseable; will rebuild from headers"
                );
                Ok(Vec::new())
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => {
            tracing::warn!(
                target: "snapshot",
                path = %path.display(),
                error = %e,
                "snapshot index unreadable; will rebuild from headers"
            );
            Ok(Vec::new())
        }
    }
}

/// Write the index file at `path` atomically.
///
/// # Errors
///
/// Returns [`crate::error::EngineError::Io`] / `ProfileWrite` on
/// serialize/write failure.
#[allow(dead_code, reason = "callers wired in later")]
pub(crate) fn write_index(path: &Path, entries: &[Snapshot]) -> Result<()> {
    let file = IndexFile {
        entries: entries.to_vec(),
    };
    let body = toml::to_string_pretty(&file)?;
    atomic_write(path, body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::types::{SnapshotId, SnapshotKind};
    use chrono::Utc;
    use ulid::Ulid;

    fn sample_snapshot(kind: SnapshotKind) -> Snapshot {
        Snapshot {
            id: SnapshotId(Ulid::new()),
            kind,
            label: None,
            taken_at: Utc::now(),
            content_hash: [0u8; 32],
            pinned: matches!(kind, SnapshotKind::Manual),
        }
    }

    #[test]
    fn round_trip_index_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.toml");
        let entries = vec![
            sample_snapshot(SnapshotKind::AutoSessionStart),
            sample_snapshot(SnapshotKind::Manual),
        ];
        write_index(&path, &entries).unwrap();

        let loaded = read_index(&path).unwrap();
        assert_eq!(loaded, entries);
    }

    #[test]
    fn read_missing_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.toml");
        assert!(read_index(&path).unwrap().is_empty());
    }

    #[test]
    fn read_corrupt_returns_empty_with_warn() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.toml");
        std::fs::write(&path, "{{{{ not toml").unwrap();
        // Should NOT propagate the parse error — caller handles rebuild.
        assert!(read_index(&path).unwrap().is_empty());
    }
}
