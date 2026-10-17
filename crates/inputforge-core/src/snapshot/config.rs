//! Snapshot subsystem configuration.

use serde::{Deserialize, Serialize};

/// Configuration for the snapshot subsystem.
///
/// Persisted as a sub-table of `AppSettings` (in `settings.toml`), so
/// users can hand-edit values without a UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Maximum number of unpinned snapshots retained per profile before
    /// FIFO eviction kicks in. Pinned snapshots are exempt.
    pub max_count: usize,

    /// When `true`, an `AutoSessionStart` snapshot is skipped if its
    /// `content_hash` equals the most recent existing snapshot.
    pub skip_if_unchanged: bool,
}

impl SnapshotConfig {
    /// Validate the supported rolling buffer size before saving settings.
    ///
    /// # Errors
    /// Returns an invalid configuration error outside the inclusive 1–100 range.
    pub fn validate(&self) -> crate::error::Result<()> {
        if !(1..=100).contains(&self.max_count) {
            return Err(crate::error::EngineError::InvalidConfig {
                reason: "Snapshot buffer size must be between 1 and 100".into(),
            });
        }
        Ok(())
    }
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            max_count: 10,
            skip_if_unchanged: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = SnapshotConfig::default();
        assert_eq!(cfg.max_count, 10);
        assert!(cfg.skip_if_unchanged);
    }

    #[test]
    fn config_serde_round_trip() {
        let cfg = SnapshotConfig {
            max_count: 25,
            skip_if_unchanged: false,
        };
        let s = toml::to_string(&cfg).unwrap();
        let back: SnapshotConfig = toml::from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }
}
