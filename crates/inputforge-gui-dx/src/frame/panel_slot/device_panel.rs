use dioxus::prelude::*;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::DeviceId;

use crate::components::{Badge, BadgeVariant};
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
                div { class: "if-device-panel__empty-copy", "Connect a controller, wheel, pedals, or other input device to populate this panel." }
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
    let usage_items = usage_count_items(&row.usage);
    let state_label = if row.connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let state_variant = if row.connected {
        BadgeVariant::Success
    } else {
        BadgeVariant::Neutral
    };
    let row_for_select = row.clone();
    let onclick = move |_| onselect.call(row_for_select.clone());
    rsx! {
        button {
            r#type: "button",
            class: if selected { "if-device-row if-device-row--selected" } else { "if-device-row" },
            "aria-pressed": "{selected}",
            onclick,
            span { class: "if-device-row__names",
                span { class: "if-device-row__headline",
                    span { class: "if-device-row__display", "{row.display_name}" }
                    Badge { variant: state_variant, "{state_label}" }
                }
                span { class: "if-device-row__hardware", title: "{row.hardware_name}", "{row.hardware_name}" }
            }
            if !usage_items.is_empty() {
                span { class: "if-device-row__counts",
                    for item in usage_items {
                        Badge {
                            variant: if item.complete { BadgeVariant::Success } else { BadgeVariant::Neutral },
                            "{item.label}"
                        }
                    }
                }
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
    let mut draft_alias_for_keydown = draft_alias;
    let mut save_error_for_save = save_error;
    let mut save_error_for_keydown = save_error;
    let mut save_error_for_copy = save_error;
    let commands = ctx.commands.clone();
    let commands_for_keydown = commands.clone();
    let save_device_for_keydown = save_device.clone();
    let persisted_alias_for_keydown = persisted_alias.clone();
    let report_for_copy = report.clone();
    let oninput = move |event: FormEvent| draft_alias_for_input.set(event.value());
    let save_click = move |_| {
        let command =
            build_set_device_alias_command(save_device.clone(), &draft_alias_for_save.read());
        if let Err(error) = commands.send(command) {
            save_error_for_save.set(Some(error.to_string()));
        } else {
            save_error_for_save.set(None);
        }
    };
    let onkeydown = move |event: KeyboardEvent| match event.key() {
        Key::Enter => {
            event.prevent_default();
            let draft_value = draft_alias_for_keydown.read().clone();
            if draft_value.trim() == persisted_alias_for_keydown {
                return;
            }
            let command =
                build_set_device_alias_command(save_device_for_keydown.clone(), &draft_value);
            if let Err(error) = commands_for_keydown.send(command) {
                save_error_for_keydown.set(Some(error.to_string()));
            } else {
                save_error_for_keydown.set(None);
            }
        }
        Key::Escape => {
            event.prevent_default();
            draft_alias_for_keydown.set(alias_draft_after_escape(&persisted_alias_for_keydown));
            save_error_for_keydown.set(None);
        }
        _ => {}
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
            div { class: "if-device-inspector__field",
                span { "Display name" }
                div { class: "if-device-inspector__edit-row",
                    input {
                        "aria-label": "Display name",
                        class: "if-device-inspector__input if-text-input if-text-input--inset",
                        value: "{draft_value}",
                        oninput,
                        onkeydown,
                    }
                    button {
                        r#type: "button",
                        class: "if-device-inspector__save if-button if-button--primary if-button--sm",
                        disabled: !dirty,
                        onclick: save_click,
                        "Save"
                    }
                }
            }
            if let Some(error) = error {
                div { class: "if-device-inspector__error", "{error}" }
            }
            div { class: "if-device-inspector__meta",
                span { class: "if-device-inspector__meta-label", "Hardware" }
                span { class: "if-device-inspector__hardware", title: "{row.hardware_name}", "{row.hardware_name}" }
            }
            UsageBlock { row: row.clone() }
            button {
                r#type: "button",
                class: "if-device-inspector__copy if-button if-button--secondary if-button--sm",
                onclick: copy_click,
                "Copy report"
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

fn alias_draft_after_escape(persisted_alias: &str) -> String {
    persisted_alias.to_owned()
}

fn build_set_device_alias_command(device: DeviceId, alias: &str) -> EngineCommand {
    let alias = alias.trim().to_owned();
    EngineCommand::SetDeviceAlias {
        device,
        alias: if alias.is_empty() { None } else { Some(alias) },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UsageCountItem {
    label: String,
    complete: bool,
}

fn usage_count_items(usage: &crate::context::DeviceUsageSummary) -> Vec<UsageCountItem> {
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
        format!("SDL GUID: {}", row.device_id.0),
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
fn UsageBlock(row: DevicePanelRow) -> Element {
    rsx! {
        div { class: "if-device-usage",
            div { class: "if-device-usage__row",
                span { class: "if-device-inspector__meta-label", "Primary mappings" }
                span { "{row.usage.primary_mappings}" }
            }
            div { class: "if-device-usage__row",
                span { class: "if-device-inspector__meta-label", "Merge/conditional refs" }
                span { "{row.usage.secondary_mappings}" }
            }
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
        assert!(report.contains("Axes: 0/4 mapped"));
        assert!(!report.contains("Profile path"));
        assert!(!report.contains("Active mode"));
        assert!(!report.contains("Instance path"));
        assert!(!report.contains("Connection:"));
        assert!(!report.contains("VID:"));
        assert!(!report.contains("PID:"));
        assert!(!report.contains("Serial:"));
    }

    #[test]
    fn device_report_omits_zero_total_usage_categories() {
        let mut row = panel_row("dev-1", "Button Box", true);
        row.usage.axes.total = 0;
        row.usage.buttons.total = 16;
        row.usage.hats.total = 0;
        let report = build_device_report(&row);

        assert!(!report.contains("Axes: 0/0 mapped"));
        assert!(report.contains("Buttons: 0/16 mapped"));
        assert!(!report.contains("Hats: 0/0 mapped"));
    }

    #[test]
    fn usage_count_labels_omit_zero_total_categories() {
        let mut row = panel_row("dev-1", "Button Box", true);
        row.usage.axes.total = 0;
        row.usage.buttons.total = 16;
        row.usage.hats.total = 0;

        let labels: Vec<_> = usage_count_items(&row.usage)
            .into_iter()
            .map(|item| (item.label, item.complete))
            .collect();

        assert_eq!(labels, vec![("Buttons 0/16".to_owned(), false)]);
    }

    #[test]
    fn usage_count_items_marks_complete_categories() {
        let mut row = panel_row("dev-1", "Button Box", true);
        row.usage.axes.mapped = 4;
        row.usage.buttons.mapped = 6;
        row.usage.buttons.total = 12;

        let labels: Vec<_> = usage_count_items(&row.usage)
            .into_iter()
            .map(|item| (item.label, item.complete))
            .collect();

        assert_eq!(
            labels,
            vec![
                ("Axes 4/4".to_owned(), true),
                ("Buttons 6/12".to_owned(), false),
                ("Hats 0/1".to_owned(), false),
            ]
        );
    }

    #[test]
    fn device_alias_command_trims_and_clears_empty_names() {
        let empty = build_set_device_alias_command(DeviceId("dev-1".to_owned()), "   ");
        let named = build_set_device_alias_command(DeviceId("dev-1".to_owned()), "  Rig Pedals  ");

        assert!(matches!(
            empty,
            EngineCommand::SetDeviceAlias { alias: None, .. }
        ));
        assert!(matches!(
            named,
            EngineCommand::SetDeviceAlias {
                alias: Some(alias),
                ..
            } if alias == "Rig Pedals"
        ));
    }

    #[test]
    fn alias_draft_comes_from_current_selection() {
        let first = panel_row_with_alias("wheel", "Wheel Base", "Rig Wheel", true);
        let second = panel_row_with_alias("pedals", "Pedals", "", true);

        assert_eq!(alias_draft_for_selected_row(&first), "Rig Wheel");
        assert_eq!(alias_draft_for_selected_row(&second), "");
    }

    #[test]
    fn escape_reverts_alias_draft_to_persisted_alias() {
        assert_eq!(alias_draft_after_escape("Rig Pedals"), "Rig Pedals");
        assert_eq!(alias_draft_after_escape(""), "");
    }

    #[test]
    fn device_panel_renders_ledger_and_fixed_inspector() {
        let html = render_device_panel(vec![panel_row("dev-1", "Wheel Base", true)]);

        assert!(html.contains("if-device-panel__ledger"));
        assert!(html.contains("Wheel Base"));
        assert!(html.contains("SDL Wheel"));
        assert!(html.contains("Axes 0/4"));
        assert!(html.contains("Display name"));
        assert!(html.contains("if-text-input"));
        assert!(html.contains("if-text-input--inset"));
        assert!(html.contains("if-device-inspector__edit-row"));
        assert!(html.contains(">Save<"));
        assert!(!html.contains("Save name"));
        assert!(
            html.contains("if-device-inspector__save if-button if-button--primary if-button--sm")
        );
        assert!(html.contains("Hardware"));
        assert!(html.contains("Merge/conditional refs"));
        assert!(html.contains("Copy report"));
        assert!(!html.contains("Copy device report"));
        assert!(
            html.contains("if-device-inspector__copy if-button if-button--secondary if-button--sm")
        );
        assert!(!html.contains(">Path<"));
        assert!(!html.contains(">Connection<"));
        assert!(!html.contains(">Type<"));
        assert!(!html.contains(">VID/PID<"));
        assert!(!html.contains(">Serial<"));
    }

    #[test]
    fn device_panel_marks_selected_row_for_interaction_state() {
        let html = render_device_panel(vec![panel_row("dev-1", "Wheel Base", true)]);

        assert!(html.contains("if-device-row if-device-row--selected"));
    }

    #[test]
    fn device_panel_renders_connection_state_on_name_line() {
        let html = render_device_panel(vec![panel_row("dev-1", "Wheel Base", true)]);

        assert!(html.contains("if-device-row__headline"));
        assert!(html.contains("if-badge--success"));
        assert!(html.contains(">Connected<"));
        let headline_index = html
            .find("if-device-row__headline")
            .expect("headline class");
        let display_index = html.find("if-device-row__display").expect("display class");
        let badge_index = html.find("if-badge--success").expect("state badge class");
        let hardware_index = html
            .find("if-device-row__hardware")
            .expect("hardware class");

        assert!(headline_index < display_index);
        assert!(display_index < badge_index);
        assert!(badge_index < hardware_index);
        assert!(!html.contains("CONNECTED"));
    }

    #[test]
    fn disconnected_row_renders_neutral_state_badge() {
        let html = render_device_panel(vec![panel_row("dev-old", "Old Pedals", false)]);

        assert!(html.contains("if-badge--neutral"));
        assert!(html.contains(">Disconnected<"));
        assert!(!html.contains("if-badge--success"));
    }

    #[test]
    fn device_panel_clamps_hardware_name_with_full_title() {
        let html = render_device_panel(vec![panel_row("dev-1", "Wheel Base", true)]);

        assert!(html.contains("if-device-row__hardware"));
        assert!(html.contains("title=\"SDL Wheel\""));
    }

    #[test]
    fn device_panel_renders_usage_count_badges_with_completion_state() {
        let mut row = panel_row("dev-1", "Wheel Base", true);
        row.usage.axes.mapped = 4;
        row.usage.buttons.mapped = 6;
        let html = render_device_panel(vec![row]);

        // The fully-mapped Axes badge uses Success; the partial Buttons
        // badge uses Neutral. The connection state also uses Success, so
        // assert the Neutral variant alongside the count text to lock the
        // mapping between completeness and badge variant.
        assert!(html.contains("if-device-row__counts"));
        assert!(html.contains("if-badge--success"));
        assert!(html.contains("if-badge--neutral"));
        assert!(html.contains("Axes 4/4"));
        assert!(html.contains("Buttons 6/12"));
    }

    #[test]
    fn panel_slot_css_defines_device_row_interaction_states() {
        let css = include_str!("../../../assets/frame/panel_slot.css");

        for selector in [
            ".if-device-row:hover:not(:disabled)",
            ".if-device-row:focus-visible",
            ".if-device-row:active:not(:disabled)",
            ".if-device-row--selected:hover:not(:disabled)",
        ] {
            assert!(css.contains(selector), "missing selector {selector}");
        }
        assert!(css.contains("cursor: pointer;"));
        assert!(css.contains("transform: translateY(1px);"));
        assert!(css.contains("transform: translateY(2px);"));
    }

    #[test]
    fn panel_slot_css_keeps_device_row_status_on_name_line() {
        let css = include_str!("../../../assets/frame/panel_slot.css");

        // The headline flex container is what places the state badge to
        // the right of the display name without a custom selector for
        // the badge itself (Badge owns its own intrinsic styling).
        assert!(css.contains(".if-device-row__headline {\n"));
        assert!(css.contains("justify-content: space-between;"));
        // Bespoke state-dot / state-label / state CSS were removed when
        // the hand-rolled indicator was replaced by Badge; their absence
        // is part of the contract.
        assert!(!css.contains(".if-device-row__state {\n"));
        assert!(!css.contains(".if-device-row__state-dot {\n"));
        assert!(!css.contains(".if-device-row__state-label {\n"));
    }

    #[test]
    fn panel_slot_css_drops_count_chip_styling_in_favor_of_badge() {
        let css = include_str!("../../../assets/frame/panel_slot.css");

        // Counts wrapper still defines the flex+gap rhythm for the row of
        // badges, but the chip-specific surface styling moved to Badge.
        assert!(css.contains(".if-device-row__counts {\n"));
        assert!(!css.contains(".if-device-row__count-chip"));
    }

    #[test]
    fn panel_slot_css_keeps_hardware_clamp() {
        let css = include_str!("../../../assets/frame/panel_slot.css");

        assert!(css.contains(".if-device-row__hardware {\n"));
        assert!(css.contains("-webkit-line-clamp: 2;"));
    }

    #[test]
    fn panel_slot_css_uses_documented_tokens_for_selected_row() {
        let css = include_str!("../../../assets/frame/panel_slot.css");

        // The previous --color-accent name had no token definition; the
        // selected-state surface is now anchored to the documented
        // border-focus + primary pair, mirroring .profile-row--active.
        assert!(!css.contains("--color-accent"));
        assert!(css.contains(".if-device-row--selected {\n"));
        assert!(css.contains(
            "    border-color: var(--color-border-focus);\n    background: color-mix(in srgb, var(--color-primary) 8%, var(--color-bg));"
        ));
    }

    #[test]
    fn panel_slot_css_pins_device_inspector_on_panel_surface() {
        // DESIGN.md §6 "Pinned Inspector" contract: stays on the
        // panel surface, separated by a 1px strong-border-top and
        // space-3 padding-top. No background declaration on the
        // inspector, so it inherits the panel's bg-elevated.
        let css = include_str!("../../../assets/frame/panel_slot.css");
        let block_start = css
            .find(".if-device-panel__inspector {\n")
            .expect("inspector rule");
        let block_end = block_start + css[block_start..].find('}').expect("inspector rule close");
        let inspector_block = &css[block_start..=block_end];

        assert!(inspector_block.contains("padding-top: var(--space-3);"));
        assert!(inspector_block.contains("border-top: 1px solid var(--color-border-strong);"));
        assert!(
            !inspector_block.contains("background:"),
            "Pinned Inspector inherits the panel surface (DESIGN.md §6); no background \
             declaration belongs on .if-device-panel__inspector. Found block:\n{inspector_block}"
        );
    }

    #[test]
    fn panel_slot_css_aligns_device_row_shape_with_profile_row() {
        let css = include_str!("../../../assets/frame/panel_slot.css");

        // DESIGN.md §1: 4px is the default radius; 2px is reserved for
        // checkbox-class controls. The Devices and Profiles rows share
        // the same row-in-side-panel shape, so they share the same
        // radius+padding tokens.
        assert!(css.contains(".if-device-row {\n"));
        assert!(css.contains("    padding: var(--space-3);\n"));
        assert!(css.contains("    border-radius: var(--radius-md);\n"));
        assert!(!css.contains("    border-radius: var(--radius-sm);\n"));
    }

    #[test]
    fn panel_slot_css_leaves_inspector_button_visuals_to_shared_button() {
        let css = include_str!("../../../assets/frame/panel_slot.css");
        let button_layout = ".if-device-inspector__save,\n.if-device-inspector__copy {\n    align-self: flex-start;\n}";

        assert!(css.contains(button_layout));
        assert!(!css.contains(
            ".if-device-inspector__save,\n.if-device-inspector__copy {\n    align-self: flex-start;\n    padding:"
        ));
    }

    #[test]
    fn panel_slot_css_keeps_device_inspector_anchored_below_scrollable_ledger() {
        let css = include_str!("../../../assets/frame/panel_slot.css");

        assert!(css.contains(".if-panel-slot__body {\n"));
        assert!(css.contains("display: flex;"));
        assert!(css.contains("overflow-y: hidden;"));
        assert!(css.contains(".if-device-panel {\n"));
        assert!(css.contains("height: 100%;"));
        assert!(css.contains("grid-template-rows: minmax(0, 1fr) auto;"));
        assert!(css.contains(".if-device-panel__ledger {\n"));
        assert!(css.contains("overflow-y: auto;"));
    }

    #[test]
    fn panel_slot_css_defines_inline_device_inspector_edit_row() {
        let css = include_str!("../../../assets/frame/panel_slot.css");

        assert!(css.contains(".if-device-inspector__edit-row {\n"));
        assert!(css.contains("grid-template-columns: minmax(0, 1fr) auto;"));
        assert!(css.contains(".if-device-inspector__meta-label {\n"));
    }

    #[test]
    fn device_panel_omits_zero_total_usage_counts() {
        let mut row = panel_row("dev-1", "Button Box", true);
        row.usage.axes.total = 0;
        row.usage.buttons.total = 0;
        row.usage.hats.total = 0;
        let html = render_device_panel(vec![row]);

        assert!(!html.contains("Axes 0/0"));
        assert!(!html.contains("Buttons 0/0"));
        assert!(!html.contains("Hats 0/0"));
        assert!(!html.contains("if-device-row__counts"));
    }

    #[test]
    fn device_panel_keeps_nonzero_usage_counts() {
        let mut row = panel_row("dev-1", "Button Box", true);
        row.usage.axes.total = 0;
        row.usage.buttons.total = 16;
        row.usage.hats.total = 0;
        let html = render_device_panel(vec![row]);

        assert!(!html.contains("Axes 0/0"));
        assert!(html.contains("Buttons 0/16"));
        assert!(!html.contains("Hats 0/0"));
    }

    #[test]
    fn device_panel_renders_no_signal_empty_state() {
        let html = render_device_panel(vec![]);

        assert!(html.contains("No devices known"));
        assert!(html.contains("Connect a controller, wheel, pedals, or other input device"));
        assert!(!html.contains("SDL"));
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
        assert!(!copied.contains("Instance path"));
        assert!(!copied.contains("Connection:"));
        assert!(!copied.contains("VID:"));
        assert!(!copied.contains("PID:"));
        assert!(!copied.contains("Serial:"));
    }
}
