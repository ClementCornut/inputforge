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
