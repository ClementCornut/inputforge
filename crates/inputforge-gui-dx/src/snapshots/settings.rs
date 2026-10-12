//! Prepared application settings for rendering.
use inputforge_core::{
    action::DEFAULT_GESTURE_THRESHOLD_MS, settings::StartupSettings, snapshot::SnapshotConfig,
};
/// Polled projection of `AppSettings.snapshot` plus the count of unpinned
/// snapshots in the active profile.
///
/// `unpinned_snapshot_count` is derived from `AppState.active_snapshot_rows`
/// (the engine's projection of the active namespace, refreshed on every
/// snapshot mutation). The count is consumed by the F15 settings panel to
/// derive `would_prune` at commit time without an additional engine query
/// channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub snapshot: SnapshotConfig,
    pub unpinned_snapshot_count: usize,
    pub startup: StartupSettings,
    pub default_double_tap_threshold_ms: u64,
    pub default_long_press_threshold_ms: u64,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            snapshot: SnapshotConfig::default(),
            unpinned_snapshot_count: 0,
            startup: StartupSettings::default(),
            default_double_tap_threshold_ms: DEFAULT_GESTURE_THRESHOLD_MS,
            default_long_press_threshold_ms: DEFAULT_GESTURE_THRESHOLD_MS,
        }
    }
}
