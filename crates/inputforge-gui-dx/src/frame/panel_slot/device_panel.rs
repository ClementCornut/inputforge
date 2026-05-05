#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Task 6 wires these pure helpers into the rendered panel"
    )
)]

use inputforge_core::types::DeviceId;

use crate::context::DevicePanelRow;

pub(super) fn select_initial_device(rows: &[DevicePanelRow]) -> Option<DeviceId> {
    rows.iter()
        .find(|row| row.connected)
        .or_else(|| rows.first())
        .map(|row| row.device_id.clone())
}

pub(super) fn alias_draft_for_selected_row(row: &DevicePanelRow) -> String {
    row.alias.clone()
}

pub(super) fn build_device_report(row: &DevicePanelRow) -> String {
    let diagnostics = &row.diagnostics;
    let serial = diagnostics.serial.as_deref().unwrap_or("unavailable");
    let instance_path = row.info.instance_path.as_deref().unwrap_or("unavailable");
    format!(
        "Display name: {}\nHardware name: {}\nConnection: {}\nAxes: {}/{} mapped\nButtons: {}/{} mapped\nHats: {}/{} mapped\nSDL GUID: {}\nVID: {}\nPID: {}\nProduct version: {}\nFirmware version: {}\nSerial: {}\nInstance path: {}",
        row.display_name,
        row.hardware_name,
        if row.connected {
            "connected"
        } else {
            "disconnected"
        },
        row.usage.axes.mapped,
        row.usage.axes.total,
        row.usage.buttons.mapped,
        row.usage.buttons.total,
        row.usage.hats.mapped,
        row.usage.hats.total,
        row.device_id.0,
        format_optional_hex(diagnostics.vendor_id),
        format_optional_hex(diagnostics.product_id),
        format_optional_u16(diagnostics.product_version),
        format_optional_u16(diagnostics.firmware_version),
        serial,
        instance_path
    )
}

fn format_optional_hex(value: Option<u16>) -> String {
    value.map_or_else(
        || "unavailable".to_owned(),
        |value| format!("0x{value:04x}"),
    )
}

fn format_optional_u16(value: Option<u16>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::context::{DeviceCoverage, DeviceUsageSummary};
    use inputforge_core::types::{AxisPolarity, DeviceDiagnostics, DeviceInfo};

    fn panel_row(id: &str, display_name: &str, connected: bool) -> DevicePanelRow {
        panel_row_with_alias(id, display_name, "", connected)
    }

    fn panel_row_with_alias(
        id: &str,
        display_name: &str,
        alias: &str,
        connected: bool,
    ) -> DevicePanelRow {
        DevicePanelRow {
            device_id: DeviceId(id.to_owned()),
            display_name: display_name.to_owned(),
            alias: alias.to_owned(),
            hardware_name: "SDL Wheel".to_owned(),
            connected,
            info: DeviceInfo {
                id: DeviceId(id.to_owned()),
                name: "SDL Wheel".to_owned(),
                axes: 4,
                buttons: 12,
                hats: 1,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 4],
            },
            diagnostics: DeviceDiagnostics::default(),
            usage: DeviceUsageSummary {
                axes: DeviceCoverage {
                    mapped: 0,
                    total: 4,
                },
                buttons: DeviceCoverage {
                    mapped: 0,
                    total: 12,
                },
                hats: DeviceCoverage {
                    mapped: 0,
                    total: 1,
                },
                primary_mappings: 0,
                secondary_mappings: 0,
                touched_modes: vec![],
                touched_mapping_names: vec![],
            },
            last_seen_unix_ms: None,
        }
    }

    #[test]
    fn select_initial_device_prefers_first_connected() {
        let rows = vec![
            panel_row("old", "Old Pedals", false),
            panel_row("live", "Wheel", true),
        ];

        assert_eq!(
            select_initial_device(&rows),
            Some(DeviceId("live".to_owned()))
        );
    }

    #[test]
    fn select_initial_device_uses_first_remembered_when_none_connected() {
        let rows = vec![panel_row("old", "Old Pedals", false)];

        assert_eq!(
            select_initial_device(&rows),
            Some(DeviceId("old".to_owned()))
        );
    }

    #[test]
    fn device_report_is_plain_text_and_device_only() {
        let row = panel_row("dev-1", "Wheel Base", true);
        let report = build_device_report(&row);

        assert!(report.contains("Display name: Wheel Base"));
        assert!(report.contains("Hardware name: SDL Wheel"));
        assert!(report.contains("Connection: connected"));
        assert!(report.contains("Axes: 0/4 mapped"));
        assert!(!report.contains("Profile path"));
        assert!(!report.contains("Active mode"));
    }

    #[test]
    fn alias_draft_comes_from_current_selection() {
        let first = panel_row_with_alias("wheel", "Wheel Base", "Rig Wheel", true);
        let second = panel_row_with_alias("pedals", "Pedals", "", true);

        assert_eq!(alias_draft_for_selected_row(&first), "Rig Wheel");
        assert_eq!(alias_draft_for_selected_row(&second), "");
    }
}
