use dioxus::prelude::*;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::DeviceId;

use crate::context::{AppContext, DevicePanelRow};

#[component]
pub(super) fn DevicePanel() -> Element {
    let ctx = use_context::<AppContext>();
    let rows = use_memo(move || ctx.config.read().device_panel_rows.clone());
    let initial_selected = select_initial_device(&rows.read());
    let initial_alias = {
        let current_rows = rows.read();
        initial_selected
            .as_ref()
            .and_then(|id| current_rows.iter().find(|row| &row.device_id == id))
            .map(alias_draft_for_selected_row)
            .unwrap_or_default()
    };
    let mut selected = use_signal(|| initial_selected.clone());
    let mut draft_alias = use_signal(|| initial_alias);
    let mut save_error = use_signal(|| None::<String>);

    let current_rows = rows.read();
    if current_rows.is_empty() {
        return rsx! {
            div { class: "if-device-panel if-device-panel--empty",
                div { class: "if-device-panel__empty-title", "No devices known" }
                div { class: "if-device-panel__empty-copy", "Connect a controller, wheel, pedals, or other SDL device to populate this panel." }
            }
        };
    }

    let selected_id = selected
        .read()
        .clone()
        .or_else(|| select_initial_device(&current_rows));
    let selected_row = selected_id
        .as_ref()
        .and_then(|id| current_rows.iter().find(|row| &row.device_id == id))
        .cloned()
        .unwrap_or_else(|| current_rows[0].clone());

    rsx! {
        div { class: "if-device-panel",
            div { class: "if-device-panel__ledger", role: "list",
                for row in current_rows.iter().cloned() {
                    DeviceLedgerRow {
                        row: row.clone(),
                        selected: row.device_id == selected_row.device_id,
                        onselect: move |row: DevicePanelRow| {
                            draft_alias.set(alias_draft_for_selected_row(&row));
                            save_error.set(None);
                            selected.set(Some(row.device_id.clone()));
                        },
                    }
                }
            }
            DeviceInspector {
                row: selected_row,
                draft_alias,
                save_error,
            }
        }
    }
}

#[component]
fn DeviceLedgerRow(
    row: DevicePanelRow,
    selected: bool,
    onselect: EventHandler<DevicePanelRow>,
) -> Element {
    let state_label = if row.connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let row_for_select = row.clone();
    let onclick = move |_| onselect.call(row_for_select.clone());
    rsx! {
        button {
            r#type: "button",
            class: if selected { "if-device-row if-device-row--selected" } else { "if-device-row" },
            "aria-pressed": "{selected}",
            onclick,
            span { class: "if-device-row__state", "data-connected": "{row.connected}", "{state_label}" }
            span { class: "if-device-row__names",
                span { class: "if-device-row__display", "{row.display_name}" }
                span { class: "if-device-row__hardware", "{row.hardware_name}" }
            }
            span { class: "if-device-row__counts",
                span { "Axes {row.usage.axes.mapped}/{row.usage.axes.total}" }
                span { "Buttons {row.usage.buttons.mapped}/{row.usage.buttons.total}" }
                span { "Hats {row.usage.hats.mapped}/{row.usage.hats.total}" }
            }
        }
    }
}

#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus event property syntax needs named handlers for separate buttons"
)]
fn DeviceInspector(
    row: DevicePanelRow,
    mut draft_alias: Signal<String>,
    mut save_error: Signal<Option<String>>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let persisted_alias = row.alias.clone();
    let draft_value = draft_alias.read().clone();
    let dirty = draft_value.trim() != persisted_alias;
    let report = build_device_report(&row);
    let error = save_error.read().clone();
    let save_device = row.device_id.clone();
    let mut draft_alias_for_input = draft_alias;
    let draft_alias_for_save = draft_alias;
    let mut save_error_for_save = save_error;
    let mut save_error_for_copy = save_error;
    let commands = ctx.commands.clone();
    let report_for_copy = report.clone();
    let oninput = move |event: FormEvent| draft_alias_for_input.set(event.value());
    let save_click = move |_| {
        let alias = draft_alias_for_save.read().trim().to_owned();
        let command = EngineCommand::SetDeviceAlias {
            device: save_device.clone(),
            alias: if alias.is_empty() { None } else { Some(alias) },
        };
        if let Err(error) = commands.send(command) {
            save_error_for_save.set(Some(error.to_string()));
        } else {
            save_error_for_save.set(None);
        }
    };
    let copy_click = move |_| {
        if let Err(error) = copy_device_report_to_clipboard(&report_for_copy) {
            save_error_for_copy.set(Some(format!("Copy failed: {error}")));
        } else {
            save_error_for_copy.set(None);
        }
    };

    rsx! {
        section { class: "if-device-panel__inspector", "aria-label": "Selected device details",
            label { class: "if-device-inspector__field",
                span { "Display name" }
                input {
                    class: "if-device-inspector__input",
                    value: "{draft_value}",
                    oninput,
                }
            }
            button {
                r#type: "button",
                class: "if-device-inspector__save",
                disabled: !dirty,
                onclick: save_click,
                "Save name"
            }
            if let Some(error) = error {
                div { class: "if-device-inspector__error", "{error}" }
            }
            div { class: "if-device-inspector__hardware", "{row.hardware_name}" }
            DiagnosticsBlock { row: row.clone() }
            UsageBlock { row: row.clone() }
            button {
                r#type: "button",
                class: "if-device-inspector__copy",
                onclick: copy_click,
                "Copy device report"
            }
        }
    }
}

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

fn copy_device_report_to_clipboard(report: &str) -> anyhow::Result<()> {
    copy_device_report_to_clipboard_with(report, |text| {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    })
}

fn copy_device_report_to_clipboard_with(
    report: &str,
    write_text: impl FnOnce(String) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    write_text(report.to_owned())
}

#[component]
fn DiagnosticsBlock(row: DevicePanelRow) -> Element {
    let connection = if row.connected {
        "connected"
    } else {
        "disconnected"
    };
    let joystick_type = row
        .diagnostics
        .joystick_type
        .as_deref()
        .unwrap_or("unknown")
        .to_owned();
    let vid = format_optional_hex(row.diagnostics.vendor_id);
    let pid = format_optional_hex(row.diagnostics.product_id);
    let serial = row
        .diagnostics
        .serial
        .as_deref()
        .unwrap_or("unavailable")
        .to_owned();
    let instance_path = row
        .info
        .instance_path
        .as_deref()
        .unwrap_or("unavailable")
        .to_owned();

    rsx! {
        dl { class: "if-device-diagnostics",
            dt { "Connection" }
            dd { "{connection}" }
            dt { "Type" }
            dd { "{joystick_type}" }
            dt { "VID/PID" }
            dd { "{vid} / {pid}" }
            dt { "Serial" }
            dd { "{serial}" }
            dt { "Path" }
            dd { "{instance_path}" }
        }
    }
}

#[component]
fn UsageBlock(row: DevicePanelRow) -> Element {
    rsx! {
        div { class: "if-device-usage",
            div { "Primary mappings {row.usage.primary_mappings}" }
            div { "Merge and conditional references {row.usage.secondary_mappings}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, mpsc};

    use crate::context::{
        ConfigSnapshot, DeviceCoverage, DeviceUsageSummary, LiveSnapshot, MetaSnapshot,
    };
    use dioxus_ssr::render;
    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{AxisPolarity, DeviceDiagnostics, DeviceInfo};
    use parking_lot::RwLock;

    #[derive(Clone, Props, PartialEq)]
    struct TestHarnessProps {
        rows: Vec<DevicePanelRow>,
    }

    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus component props are passed by value"
    )]
    fn TestHarness(props: TestHarnessProps) -> Element {
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, _rx) = mpsc::channel();
        let settings = Arc::new(AppSettings::default());
        let meta = use_signal(MetaSnapshot::default);
        let config = ConfigSnapshot {
            device_panel_rows: props.rows.clone(),
            ..Default::default()
        };
        let config = use_signal(|| config);
        let live = use_signal(LiveSnapshot::default);

        use_context_provider(|| AppContext {
            state,
            commands,
            settings,
            meta,
            config,
            live,
        });

        rsx! { DevicePanel {} }
    }

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

    fn render_device_panel(rows: Vec<DevicePanelRow>) -> String {
        let mut vdom = VirtualDom::new_with_props(TestHarness, TestHarnessProps { rows });
        vdom.rebuild_in_place();
        render(&vdom)
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

    #[test]
    fn device_panel_renders_ledger_and_fixed_inspector() {
        let html = render_device_panel(vec![panel_row("dev-1", "Wheel Base", true)]);

        assert!(html.contains("if-device-panel__ledger"));
        assert!(html.contains("Wheel Base"));
        assert!(html.contains("SDL Wheel"));
        assert!(html.contains("Axes 0/4"));
        assert!(html.contains("Display name"));
        assert!(html.contains("Save name"));
        assert!(html.contains("Copy device report"));
    }

    #[test]
    fn device_panel_renders_no_signal_empty_state() {
        let html = render_device_panel(vec![]);

        assert!(html.contains("No devices known"));
        assert!(!html.contains("if-device-panel__inspector"));
    }

    #[test]
    fn disconnected_row_keeps_profile_counts_visible() {
        let html = render_device_panel(vec![panel_row("dev-old", "Remembered Pedals", false)]);

        assert!(html.contains("Disconnected"));
        assert!(html.contains("Axes 0/4"));
    }

    #[test]
    fn copy_device_report_helper_forwards_report_text() {
        let row = panel_row("dev-1", "Wheel Base", true);
        let report = build_device_report(&row);
        let mut copied = None::<String>;

        copy_device_report_to_clipboard_with(&report, |text| {
            copied = Some(text);
            Ok(())
        })
        .expect("copy helper succeeds");

        let copied = copied.expect("copied text");
        assert!(copied.contains("Display name: Wheel Base"));
        assert!(copied.contains("Hardware name: SDL Wheel"));
    }
}
