//! Project application settings and snapshot counts.
use inputforge_core::state::AppState;
use inputforge_gui_dx::snapshots::SettingsSnapshot;
/// Project an `AppState` into a `SettingsSnapshot`.
///
/// Reads `state.snapshot_config` (the engine's mirror of
/// `AppSettings.snapshot`, populated on every settings mutation) and
/// counts unpinned entries in `state.active_snapshot_rows` (refreshed
/// by the engine after every snapshot-mutating command). No filesystem
/// IO is performed; this runs on every polling tick.
pub(super) fn project(state: &AppState) -> SettingsSnapshot {
    let snapshot = state.snapshot_config.clone();
    let unpinned_snapshot_count = state
        .active_snapshot_rows
        .iter()
        .filter(|row| !row.pinned)
        .count();
    let startup = state.startup.clone();
    SettingsSnapshot {
        snapshot,
        unpinned_snapshot_count,
        startup,
        default_double_tap_threshold_ms: state.default_double_tap_threshold_ms,
        default_long_press_threshold_ms: state.default_long_press_threshold_ms,
    }
}
