//! Pure device usage labels and plain-text report formatting.
use crate::context::DevicePanelRow;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UsageCountItem {
    pub(super) label: String,
    pub(super) complete: bool,
}

pub(super) fn usage_count_items(usage: &crate::context::DeviceUsageSummary) -> Vec<UsageCountItem> {
    let mut labels = Vec::new();
    if usage.axes.total > 0 {
        labels.push(UsageCountItem {
            label: format!("Axes {}/{}", usage.axes.mapped, usage.axes.total),
            complete: usage.axes.mapped == usage.axes.total,
        });
    }
    if usage.buttons.total > 0 {
        labels.push(UsageCountItem {
            label: format!("Buttons {}/{}", usage.buttons.mapped, usage.buttons.total),
            complete: usage.buttons.mapped == usage.buttons.total,
        });
    }
    if usage.hats.total > 0 {
        labels.push(UsageCountItem {
            label: format!("Hats {}/{}", usage.hats.mapped, usage.hats.total),
            complete: usage.hats.mapped == usage.hats.total,
        });
    }
    labels
}

fn usage_report_lines(usage: &crate::context::DeviceUsageSummary) -> Vec<String> {
    let mut lines = Vec::new();
    if usage.axes.total > 0 {
        lines.push(format!(
            "Axes: {}/{} mapped",
            usage.axes.mapped, usage.axes.total
        ));
    }
    if usage.buttons.total > 0 {
        lines.push(format!(
            "Buttons: {}/{} mapped",
            usage.buttons.mapped, usage.buttons.total
        ));
    }
    if usage.hats.total > 0 {
        lines.push(format!(
            "Hats: {}/{} mapped",
            usage.hats.mapped, usage.hats.total
        ));
    }
    lines
}

pub(super) fn build_device_report(row: &DevicePanelRow) -> String {
    let diagnostics = &row.diagnostics;
    let mut lines = vec![
        format!("Display name: {}", row.display_name),
        format!("Hardware name: {}", row.hardware_name),
    ];
    lines.extend(usage_report_lines(&row.usage));
    lines.extend([
        format!("Device ID: {}", row.device_id.0),
        format!(
            "Product version: {}",
            format_optional_u16(diagnostics.product_version)
        ),
        format!(
            "Firmware version: {}",
            format_optional_u16(diagnostics.firmware_version)
        ),
    ]);
    lines.join("\n")
}

fn format_optional_u16(value: Option<u16>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
}
