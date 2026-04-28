//! Snapshot configuration — populated in Task 4.

use serde::{Deserialize, Serialize};

/// User-configurable snapshot defaults and behavior.
///
/// This struct is populated in Task 4 with rolling buffer size,
/// skip-if-unchanged settings, and retention policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotConfig;
