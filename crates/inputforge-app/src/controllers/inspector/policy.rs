//! Device selection, alias and removal decisions.
use inputforge_core::state::AppState;
use inputforge_gui_dx::{controllers::inspector::View, snapshots::DevicePanelRow};
pub(super) fn first(rows: &[DevicePanelRow]) -> Option<&DevicePanelRow> {
    rows.iter().find(|r| r.connected).or_else(|| rows.first())
}
pub(super) fn availability(view: &mut View, state: &AppState) {
    view.can_save = view
        .selected
        .as_ref()
        .is_some_and(|r| view.draft.trim() != r.alias);
    view.removal_reason = view
        .selected
        .as_ref()
        .and_then(|r| {
            if state.active_profile.is_none() {
                Some("Load a profile to remove a device.")
            } else if r.connected {
                Some("Disconnect this device before removing it.")
            } else if r.usage.primary_mappings > 0 || r.usage.secondary_mappings > 0 {
                Some("Remove this device's mapping references first.")
            } else {
                None
            }
        })
        .map(str::to_owned);
}
