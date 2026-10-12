//! Availability and layout presentation derived from engine-confirmed state.
use inputforge_core::state::{AppState, EngineStatus};
use inputforge_gui_dx::{
    controllers::{Selection, View},
    snapshots::DevicePanelRow,
};

pub(super) fn editable(state: &AppState) -> bool {
    state.active_profile.is_some()
        && !state.session.offline
        && state.session.captured.is_empty()
        && !matches!(
            state.engine_status,
            EngineStatus::Running | EngineStatus::Starting
        )
}

/// Prepare controller values using the bridge's existing device rows.
#[must_use]
pub fn view(state: &AppState, rows: &[DevicePanelRow]) -> View {
    let editable = editable(state);
    let capabilities = state.session.controller_capabilities;
    let layout_editable = editable && capabilities.configurable;
    let config = state.active_profile.as_ref().and_then(|p| p.controllers());
    let requested = config.map_or(&[][..], |c| c.virtual_devices.as_slice());
    let pending = state.session.requires_output_reconfiguration(requested);
    View {
        editable,
        layout_editable,
        pending,
        can_apply: editable && pending,
        can_add: layout_editable && requested.len() < usize::from(super::MAX_SLOTS),
        retained: state.engine_status == EngineStatus::Stopped && state.session.output_active,
        capabilities,
        selections: rows
            .iter()
            .map(|row| {
                let selected = config.is_some_and(|c| c.selected.contains(&row.device_id));
                Selection {
                    device: row.device_id.clone(),
                    label: row.display_name.clone(),
                    connected: row.connected,
                    selected,
                    enabled: editable && (row.connected || selected),
                }
            })
            .collect(),
        slots: if capabilities.configurable {
            requested.to_vec()
        } else {
            state.virtual_devices.clone()
        },
    }
}
