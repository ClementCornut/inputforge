//! Modification-time cache for synthesized active-profile rows.
use inputforge_gui_dx::snapshots::MetaSnapshot;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

// Avoid a filesystem read on every 16 ms tick while still noticing external edits.
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);
#[derive(Default)]
pub(super) struct Cache {
    entry: Option<Entry>,
}
struct Entry {
    path: PathBuf,
    checked: Instant,
    modified: Option<SystemTime>,
}
impl Cache {
    pub(super) fn complete(
        &mut self,
        meta: &mut MetaSnapshot,
        now: Instant,
        read: impl FnOnce(&Path) -> Option<SystemTime>,
    ) {
        let (Some(path), Some(row)) = (
            meta.profile_path.as_ref(),
            meta.profile_rows
                .iter_mut()
                .find(|row| row.is_active && !row.can_open),
        ) else {
            self.entry = None;
            return;
        };
        if self.entry.as_ref().is_none_or(|entry| {
            entry.path != *path || now.saturating_duration_since(entry.checked) >= REFRESH_INTERVAL
        }) {
            self.entry = Some(Entry {
                path: path.clone(),
                checked: now,
                modified: read(path),
            });
        }
        row.last_edited_label = self
            .entry
            .as_ref()
            .and_then(|entry| entry.modified)
            .map(|time| super::time_labels::format_chrono_time_relative(time.into()));
    }
}
