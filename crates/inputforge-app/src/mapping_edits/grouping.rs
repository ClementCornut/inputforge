//! Keyboard burst boundaries shared by curve and deadzone dispatch.
use inputforge_gui_dx::{mapping_edits::Group, snapshots::MappingKey};
use std::time::{Duration, Instant};
// Retain the editor's 250 ms same-key repeat window.
const WINDOW: Duration = Duration::from_millis(250);
#[derive(Debug, Clone)]
pub(super) struct Burst {
    pub key: MappingKey,
    pub group: Group,
    pub at: Instant,
}
impl Burst {
    pub(super) fn matches(&self, key: &MappingKey, group: &Group, now: Instant) -> bool {
        &self.key == key
            && &self.group == group
            && now
                .checked_duration_since(self.at)
                .is_some_and(|d| d <= WINDOW)
    }
}
