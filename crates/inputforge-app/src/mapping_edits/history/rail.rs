//! Forward rail invalidation preserves queued command membership transitions.
use super::History;
use super::saved;
use inputforge_core::{action::Mapping, state::AppState};
use inputforge_gui_dx::snapshots::MappingKey;
impl History {
    pub(in crate::mapping_edits) fn membership_pending(
        &self,
        state: &AppState,
        mode: &str,
    ) -> bool {
        self.pending.iter().any(|(key, pending)| {
            key.0 == mode
                && (saved(state, key).is_some() != self.current(state, key).is_some()
                    || pending
                        .sent
                        .iter()
                        .any(|delivery| delivery.mapping.is_some() != pending.observed.is_some()))
        })
    }
    pub(in crate::mapping_edits) fn forward(
        &mut self,
        state: &AppState,
        key: &MappingKey,
        mapping: Option<Mapping>,
        serial: u64,
    ) {
        // Forget restore snapshots, but keep acknowledgements for already queued commands.
        let pending = self.pending.remove(key);
        self.forget(key);
        if let Some(pending) = pending {
            self.pending.insert(key.clone(), pending);
        }
        self.delivered(state, key, mapping, serial, true);
    }
}
