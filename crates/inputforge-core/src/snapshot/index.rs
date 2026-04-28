//! `index.toml` cache: read, write, rebuild from snapshot file headers.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::fs::atomic_write;
use super::types::Snapshot;

#[derive(Debug, Serialize, Deserialize, Default)]
struct IndexFile {
    #[serde(default)]
    entries: Vec<Snapshot>,
}

/// Read the index file at `path`. Returns an empty vec if the file is
/// missing, unparseable, or truncated — these conditions are recoverable
/// by a rebuild from snapshot file headers, performed by the caller.
///
/// # Errors
///
/// Returns `Err` only for unexpected I/O failures; missing or corrupt
/// index files are treated as empty and do not propagate an error.
#[allow(
    clippy::unnecessary_wraps,
    reason = "Result<Vec<>> signals operation completion; the Ok(Vec::new()) \
              branches are intentional — callers treat empty-vec as rebuild trigger"
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
pub(crate) fn write_index(path: &Path, entries: &[Snapshot]) -> Result<()> {
    let file = IndexFile {
        entries: entries.to_vec(),
    };
    let body = toml::to_string_pretty(&file)?;
    atomic_write(path, body.as_bytes())
}

/// Walk `<stem>.snapshots/` and reconstruct the entries list from each
/// `<id>.toml`'s `[snapshot_meta]` header.
///
/// Files whose `[snapshot_meta]` header is missing or malformed are
/// logged and skipped (treated as deleted for `prune` purposes).
/// Files with a valid header are included in the returned list.
///
/// # Errors
///
/// Returns [`crate::error::EngineError::SnapshotDirIo`] if the
/// directory cannot be read.
pub(crate) fn rebuild_from_dir(dir: &Path) -> Result<Vec<Snapshot>> {
    let mut out = Vec::new();
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(source) => {
            return Err(crate::error::EngineError::SnapshotDirIo {
                path: dir.to_path_buf(),
                source,
            });
        }
    };
    for entry in read.flatten() {
        let path = entry.path();
        // Case-insensitive `.toml` match so paths on case-insensitive
        // filesystems (Windows, macOS default) don't disagree with the
        // orphan check in `list::count_orphans`, which would otherwise
        // cause perpetual rebuilds.
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
        {
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) == Some("index.toml") {
            continue;
        }
        let body = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    target: "snapshot",
                    path = %path.display(),
                    error = %e,
                    "snapshot file unreadable; skipping"
                );
                continue;
            }
        };
        match toml::from_str::<MetaProbe>(&body) {
            Ok(probe) => out.push(probe.snapshot_meta),
            Err(e) => {
                tracing::warn!(
                    target: "snapshot",
                    path = %path.display(),
                    error = %e,
                    "snapshot meta malformed; skipping"
                );
            }
        }
    }
    sort_newest_first(&mut out);
    Ok(out)
}

fn sort_newest_first(entries: &mut [Snapshot]) {
    entries.sort_by(|a, b| {
        b.taken_at
            .cmp(&a.taken_at)
            .then_with(|| b.id.0.cmp(&a.id.0))
    });
}

#[derive(Deserialize)]
struct MetaProbe {
    snapshot_meta: Snapshot,
}

pub(crate) fn ensure_sorted_newest_first(entries: &mut [Snapshot]) {
    sort_newest_first(entries);
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
