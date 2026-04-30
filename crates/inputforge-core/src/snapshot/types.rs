//! Snapshot data types: id, kind, full record.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// A unique, sortable snapshot identifier (ULID-based).
///
/// ULIDs are lexicographically sortable by creation time, but `list()`
/// orders by `taken_at` (descending) for user-visible ordering, the
/// ULID sort is a secondary tiebreaker only when `taken_at` collides
/// at millisecond precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub Ulid);

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// What triggered a snapshot's creation.
///
/// `Manual` is auto-pinned at creation; the auto kinds are not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotKind {
    /// Created by `LoadProfile`. Deduped against the latest snapshot when
    /// `cfg.skip_if_unchanged` is set and the content hash matches.
    AutoSessionStart,
    /// Created by `RestoreSnapshot` immediately before applying the
    /// restore. Always fires; never deduped.
    AutoBeforeRestore,
    /// Created by user dispatch of `CreateSnapshot { kind: Manual }`.
    /// Auto-pinned.
    Manual,
}

/// A snapshot record as stored in `[snapshot_meta]` and in the index cache.
///
/// `content_hash` is BLAKE3 of the canonical-round-tripped profile TOML
/// body (decision D14): `blake3(toml::to_string(toml::from_str(profile_bytes)?)?)`.
/// This makes the hash stable across whitespace, comment placement, and
/// top-level key reordering in the on-disk profile file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub kind: SnapshotKind,
    pub label: Option<String>,
    pub taken_at: DateTime<Utc>,
    /// BLAKE3 of canonical TOML, see module-level docs.
    #[serde(with = "hex_array_32")]
    pub content_hash: [u8; 32],
    pub pinned: bool,
}

mod hex_array_32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(bytes: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        let mut out = String::with_capacity(64);
        for b in bytes {
            use std::fmt::Write;
            let _ = write!(out, "{b:02x}");
        }
        s.serialize_str(&out)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let s = String::deserialize(d)?;
        if s.len() != 64 {
            return Err(serde::de::Error::custom(format!(
                "content_hash must be 64 hex chars, got {}",
                s.len()
            )));
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            let pair = &s[i * 2..i * 2 + 2];
            *byte = u8::from_str_radix(pair, 16)
                .map_err(|e| serde::de::Error::custom(format!("invalid hex at byte {i}: {e}")))?;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_id_serde_round_trip() {
        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            id: SnapshotId,
        }
        let id = SnapshotId(Ulid::new());
        let wrapper = Wrapper { id };
        let s = toml::to_string(&wrapper).unwrap();
        let back: Wrapper = toml::from_str(&s).unwrap();
        assert_eq!(back.id, id);
    }

    #[test]
    fn snapshot_kind_toml_uses_snake_case() {
        #[derive(Serialize)]
        struct Wrapper {
            kind: SnapshotKind,
        }
        let wrapper = Wrapper {
            kind: SnapshotKind::AutoSessionStart,
        };
        let s = toml::to_string(&wrapper).unwrap();
        assert!(s.contains("auto_session_start"), "got: {s}");
    }

    #[test]
    fn snapshot_record_serde_round_trip() {
        let snap = Snapshot {
            id: SnapshotId(Ulid::new()),
            kind: SnapshotKind::Manual,
            label: Some("my label".to_owned()),
            taken_at: Utc::now(),
            content_hash: [0u8; 32],
            pinned: true,
        };
        let s = toml::to_string(&snap).unwrap();
        let back: Snapshot = toml::from_str(&s).unwrap();
        assert_eq!(snap, back);
    }
}
