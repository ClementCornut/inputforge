//! Prepare live and remembered device rows and names.
use super::device_usage::usage_for_device;
use inputforge_core::state::AppState;
use inputforge_core::types::DeviceId;
use inputforge_gui_dx::snapshots::DevicePanelRow;
use std::collections::{HashMap, HashSet};
pub(crate) fn build_device_panel_rows(s: &AppState) -> Vec<DevicePanelRow> {
    let mut rows = Vec::new();
    let mut live_ids = HashSet::new();

    for device in &s.devices {
        live_ids.insert(device.info.id.clone());
        rows.push(DevicePanelRow {
            device_id: device.info.id.clone(),
            display_name: inputforge_core::settings::display_name_for_device(
                &s.device_aliases,
                &device.info,
            ),
            alias: s
                .device_aliases
                .get(&device.info.id)
                .cloned()
                .unwrap_or_default(),
            hardware_name: device.info.name.clone(),
            connected: device.connected,
            info: device.info.clone(),
            diagnostics: device.diagnostics.clone(),
            usage: usage_for_device(&device.info.id, &device.info, s),
            last_seen_unix_ms: s
                .device_registry
                .get(&device.info.id)
                .and_then(|record| record.last_seen_unix_ms),
        });
    }

    for (device_id, record) in &s.device_registry {
        if live_ids.contains(device_id) {
            continue;
        }
        rows.push(DevicePanelRow {
            device_id: device_id.clone(),
            display_name: inputforge_core::settings::display_name_for_device(
                &s.device_aliases,
                &record.info,
            ),
            alias: s.device_aliases.get(device_id).cloned().unwrap_or_default(),
            hardware_name: record.info.name.clone(),
            connected: false,
            info: record.info.clone(),
            diagnostics: record.diagnostics.clone(),
            usage: usage_for_device(device_id, &record.info, s),
            last_seen_unix_ms: record.last_seen_unix_ms,
        });
    }

    if let Some(profile) = &s.active_profile {
        rows.retain(|row| {
            row.connected
                || !profile.removed_devices().contains(&row.device_id)
                || row.usage.primary_mappings > 0
                || row.usage.secondary_mappings > 0
        });
    }

    rows.sort_by(|a, b| {
        b.connected.cmp(&a.connected).then_with(|| {
            a.display_name
                .to_ascii_lowercase()
                .cmp(&b.display_name.to_ascii_lowercase())
        })
    });
    rows
}
/// Build the precomputed `DeviceId -> display_name` map. Inserts
/// remembered devices first (`s.device_registry`), then connected
/// devices (`s.devices`), so a live entry overwrites a stale
/// remembered entry on key collision. The same device can therefore
/// appear in both maps; live identity wins.
pub(super) fn build_device_display_names(s: &AppState) -> HashMap<DeviceId, String> {
    let mut out = HashMap::new();
    for (id, record) in &s.device_registry {
        out.insert(
            id.clone(),
            inputforge_core::settings::display_name_for_device(&s.device_aliases, &record.info),
        );
    }
    for device in &s.devices {
        out.insert(
            device.info.id.clone(),
            inputforge_core::settings::display_name_for_device(&s.device_aliases, &device.info),
        );
    }
    out
}
