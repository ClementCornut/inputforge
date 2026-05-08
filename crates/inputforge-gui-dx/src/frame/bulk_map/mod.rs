//! F-bulk-map: primary workspace bulk mapping wizard. See
//! `docs/superpowers/specs/2026-05-03-bulk-mapping-design.md`.

mod apply;
mod auto_map;
mod conflicts;
mod empty_state;
mod group_actions;
mod row_readout;
mod state;
mod summary;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;
use inputforge_core::engine::EngineCommand;
use inputforge_core::state::DeviceState;
use inputforge_core::types::{
    DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis, VirtualDeviceConfig,
};

use crate::components::{Button, ButtonVariant, Checkbox, Field, Select, SelectOption};
use crate::context::{AppContext, ConfigSnapshot};
use crate::frame::bulk_map::auto_map::{auto_axis_target, auto_button_target, auto_hat_target};
use crate::frame::bulk_map::empty_state::NoVjoyEmptyState;
use crate::frame::bulk_map::group_actions::{
    show_exclude_all, show_include_all, show_replace_all_conflicts, show_skip_all_conflicts,
};
use crate::frame::bulk_map::row_readout::RowReadout;
use crate::frame::bulk_map::state::{RowKind, RowState, WizardState};
use crate::frame::view_state::{MainSurface, ViewState};
use crate::toast::{ToastLevel, ToastQueue};

const BULK_MAP_CSS: Asset = asset!("/assets/frame/bulk_map.css");

/// Build the `(id_string, display_label)` pairs for the source-device
/// dropdown. The label routes through
/// [`ConfigSnapshot::device_display_name`] so the user sees their
/// alias, falling through to the hardware name and then the id
/// string per the standard precedence. Extracted as a pure helper so
/// the alias contract can be unit-tested without a Dioxus runtime,
/// per the device-alias display-name spec.
pub(crate) fn build_source_options(
    connected: &[DeviceState],
    cfg: &ConfigSnapshot,
) -> Vec<SelectOption> {
    connected
        .iter()
        .map(|device| SelectOption {
            value: device.info.id.0.clone(),
            label: cfg.device_display_name(&device.info.id),
            disabled: false,
            class: None,
        })
        .collect()
}

#[component]
pub(crate) fn BulkMapPanel() -> Element {
    tracing::trace!(target: "frame::render", region = "bulk_map");

    let ctx = use_context::<AppContext>();

    let virtual_devices: Vec<VirtualDeviceConfig> = ctx.state.read().virtual_devices.clone();
    let has_profile = ctx.state.read().active_profile.is_some();

    if !has_profile {
        return rsx! {
            Stylesheet { href: BULK_MAP_CSS }
            section { class: "if-bulk-map", "aria-label": "Batch map device inputs",
                NoVjoyEmptyState {
                    title: "No profile loaded".to_owned(),
                    caption: "Load or create a profile, then reopen.".to_owned(),
                }
                footer { class: "if-bulk-map__footer",
                    Button { disabled: true, onclick: move |_| {}, "Apply" }
                }
            }
        };
    }

    if virtual_devices.is_empty() {
        return rsx! {
            Stylesheet { href: BULK_MAP_CSS }
            section { class: "if-bulk-map", "aria-label": "Batch map device inputs",
                NoVjoyEmptyState {}
                footer { class: "if-bulk-map__footer",
                    Button { disabled: true, onclick: move |_| {}, "Apply" }
                }
            }
        };
    }

    rsx! {
        Stylesheet { href: BULK_MAP_CSS }
        BulkMapReadyPanel {}
    }
}

#[component]
fn BulkMapReadyPanel() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let toast = use_context::<ToastQueue>();
    let main_surface = view.main_surface;

    let connected_devices = {
        let state = ctx.state.read();
        state
            .devices
            .iter()
            .filter(|device| device.connected)
            .cloned()
            .collect::<Vec<_>>()
    };
    let virtual_devices: Vec<VirtualDeviceConfig> = ctx.state.read().virtual_devices.clone();
    let editing_mode = view.editing_mode.read().clone();
    let modes: Vec<String> = ctx.meta.read().modes.clone();

    let mut wizard = use_signal(|| {
        let mut state = WizardState::empty(editing_mode.clone());
        state.source_device_id = connected_devices
            .first()
            .map(|device| device.info.id.clone());
        state.target_vjoy_id = virtual_devices.first().map(|device| device.device_id);
        state.rows = derive_rows_for_selection(
            state.source_device_id.as_ref(),
            &connected_devices,
            state.target_vjoy_id,
            &virtual_devices,
        );
        state
    });
    reconcile_wizard_state(&mut wizard.write(), &connected_devices, &virtual_devices);

    let source_text = wizard
        .peek()
        .source_device_id
        .as_ref()
        .map(|device| device.0.clone())
        .unwrap_or_default();
    let target_text = wizard
        .peek()
        .target_vjoy_id
        .map(|id| id.to_string())
        .unwrap_or_default();
    let mode_text = wizard.peek().mode.clone();
    let apply_to_all_text = wizard.peek().apply_to_all_modes;

    let mut source_value = use_signal(|| source_text.clone());
    let mut target_value = use_signal(|| target_text.clone());
    let mut mode_value = use_signal(|| mode_text.clone());
    let mut apply_to_all = use_signal(|| apply_to_all_text);
    if source_value.peek().as_str() != source_text.as_str() {
        source_value.set(source_text.clone());
    }
    if target_value.peek().as_str() != target_text.as_str() {
        target_value.set(target_text.clone());
    }

    let on_source_change = {
        let connected_devices = connected_devices.clone();
        let virtual_devices = virtual_devices.clone();
        move |evt: FormEvent| {
            let value = evt.value();
            source_value.set(value.clone());
            let source_id = DeviceId(value);
            let mut state = wizard.write();
            state.source_device_id = Some(source_id.clone());
            state.rows = derive_rows_for_selection(
                state.source_device_id.as_ref(),
                &connected_devices,
                state.target_vjoy_id,
                &virtual_devices,
            );
        }
    };
    let on_target_change = {
        let connected_devices = connected_devices.clone();
        let virtual_devices = virtual_devices.clone();
        move |evt: FormEvent| {
            let value = evt.value();
            target_value.set(value.clone());
            if let Ok(id) = value.parse::<u8>() {
                let mut state = wizard.write();
                state.target_vjoy_id = Some(id);
                state.rows = derive_rows_for_selection(
                    state.source_device_id.as_ref(),
                    &connected_devices,
                    state.target_vjoy_id,
                    &virtual_devices,
                );
            }
        }
    };
    let on_mode_change = move |evt: FormEvent| {
        let value = evt.value();
        mode_value.set(value.clone());
        wizard.write().mode = value;
    };
    let on_apply_to_all_change = move |_evt: FormEvent| {
        let checked = !*apply_to_all.peek();
        apply_to_all.set(checked);
        wizard.write().apply_to_all_modes = checked;
    };

    let source_ro: ReadSignal<String> = source_value.into();
    let target_ro: ReadSignal<String> = target_value.into();
    let mode_ro: ReadSignal<String> = mode_value.into();
    let apply_to_all_ro: ReadSignal<bool> = apply_to_all.into();

    let source_options = build_source_options(&connected_devices, &ctx.config.read());
    let target_options = virtual_devices
        .iter()
        .map(|device| SelectOption {
            value: device.device_id.to_string(),
            label: format!(
                "vJoy {}: {} axes, {} buttons, {} hat{}",
                device.device_id,
                device.axes.len(),
                device.button_count,
                device.hat_count,
                if device.hat_count == 1 { "" } else { "s" },
            ),
            disabled: false,
            class: None,
        })
        .collect::<Vec<_>>();
    let mode_options = modes
        .iter()
        .map(|mode| SelectOption {
            value: mode.clone(),
            label: mode.clone(),
            disabled: false,
            class: None,
        })
        .collect::<Vec<_>>();
    let snapshot_caption = "Snapshot taken before apply.";
    let target_for_groups = wizard.peek().target_vjoy_id.and_then(|id| {
        virtual_devices
            .iter()
            .find(|device| device.device_id == id)
            .cloned()
    });
    let rows = wizard.read().rows.clone();
    let baseline_rows = derive_rows_for_selection(
        wizard.peek().source_device_id.as_ref(),
        &connected_devices,
        wizard.peek().target_vjoy_id,
        &virtual_devices,
    );
    let is_dirty = rows_dirty(&rows, &baseline_rows);
    let axis_rows = rows
        .iter()
        .filter(|row| row.kind == RowKind::Axis)
        .cloned()
        .collect::<Vec<_>>();
    let button_rows = rows
        .iter()
        .filter(|row| row.kind == RowKind::Button)
        .cloned()
        .collect::<Vec<_>>();
    let hat_rows = rows
        .iter()
        .filter(|row| row.kind == RowKind::Hat)
        .cloned()
        .collect::<Vec<_>>();
    let on_axis_change = row_change_handler(wizard, RowKind::Axis);
    let on_button_change = row_change_handler(wizard, RowKind::Button);
    let on_hat_change = row_change_handler(wizard, RowKind::Hat);
    let on_axis_replace = row_replace_handler(wizard, RowKind::Axis);
    let on_button_replace = row_replace_handler(wizard, RowKind::Button);
    let on_hat_replace = row_replace_handler(wizard, RowKind::Hat);
    let active_modes = if *apply_to_all.read() {
        modes.clone()
    } else {
        vec![wizard.read().mode.clone()]
    };
    let (axis_conflicting, button_conflicting, hat_conflicting) = {
        let state = ctx.state.read();
        if let Some(profile) = state.active_profile.as_ref() {
            (
                row_conflicts(&axis_rows, profile, &active_modes),
                row_conflicts(&button_rows, profile, &active_modes),
                row_conflicts(&hat_rows, profile, &active_modes),
            )
        } else {
            (
                vec![false; axis_rows.len()],
                vec![false; button_rows.len()],
                vec![false; hat_rows.len()],
            )
        }
    };
    let axis_chip_handlers = group_chip_handlers(
        wizard,
        RowKind::Axis,
        target_for_groups.clone(),
        conflicting_indices(&axis_rows, &axis_conflicting),
    );
    let button_chip_handlers = group_chip_handlers(
        wizard,
        RowKind::Button,
        target_for_groups.clone(),
        conflicting_indices(&button_rows, &button_conflicting),
    );
    let hat_chip_handlers = group_chip_handlers(
        wizard,
        RowKind::Hat,
        target_for_groups.clone(),
        conflicting_indices(&hat_rows, &hat_conflicting),
    );
    let counts = {
        let state = ctx.state.read();
        let profile = state
            .active_profile
            .as_ref()
            .expect("no-profile guard at top of component covers this path");
        summary::tally(profile, &wizard.read().rows, &active_modes)
    };
    let apply_count = counts.create + counts.replace;
    let apply_label = format!("Apply {apply_count} mappings");
    let has_axis_rows = !axis_rows.is_empty();
    let has_button_rows = !button_rows.is_empty();
    let has_hat_rows = !hat_rows.is_empty();
    let has_any_rows = has_axis_rows || has_button_rows || has_hat_rows;
    let on_reset = {
        let baseline_rows = baseline_rows.clone();
        move |_| wizard.write().rows.clone_from(&baseline_rows)
    };
    let on_apply = {
        let cmd_tx = ctx.commands.clone();
        let ctx = ctx.clone();
        let mut main_surface = main_surface;
        let active_modes = active_modes.clone();
        move |_| {
            let state = wizard.peek().clone();
            let entries = {
                let app_state = ctx.state.read();
                let profile = app_state.active_profile.as_ref().expect("profile loaded");
                apply::build_entries(profile, &state.rows, &active_modes)
            };
            let source_display_name = state.source_device_id.as_ref().map_or_else(
                || "source".to_owned(),
                |id| ctx.config.read().device_display_name(id),
            );
            let count = entries.len();
            let snapshot_label = apply::format_snapshot_label(
                &source_display_name,
                state.target_vjoy_id.unwrap_or(0),
            );
            let _ = cmd_tx.send(EngineCommand::SetMappingsBulk {
                entries,
                snapshot_label,
            });
            toast.push(ToastLevel::Success, format!("Created {count} mappings"));
            main_surface.set(MainSurface::Mappings);
        }
    };

    rsx! {
        section { class: "if-bulk-map", "aria-label": "Batch map device inputs",
            div { class: "if-bulk-map__metadata",
                Field { label: "Source".to_owned(), for_id: Some("bulk-map-source".to_owned()),
                    Select {
                        id: Some("bulk-map-source".to_owned()),
                        value: source_ro,
                        onchange: on_source_change,
                        options: source_options,
                    }
                }
                Field { label: "Target".to_owned(), for_id: Some("bulk-map-target".to_owned()),
                    Select {
                        id: Some("bulk-map-target".to_owned()),
                        value: target_ro,
                        onchange: on_target_change,
                        options: target_options,
                    }
                }
                Field { label: "Mode".to_owned(), for_id: Some("bulk-map-mode".to_owned()),
                    Select {
                        id: Some("bulk-map-mode".to_owned()),
                        disabled: *apply_to_all.read(),
                        value: mode_ro,
                        onchange: on_mode_change,
                        options: mode_options,
                    }
                }
                Field {
                    class: Some("if-bulk-map__apply-field".to_owned()),
                    label: "Apply to".to_owned(),
                    for_id: Some("bulk-map-all-modes".to_owned()),
                    div { class: "if-bulk-map__apply-control",
                        Checkbox {
                            id: Some("bulk-map-all-modes".to_owned()),
                            checked: apply_to_all_ro,
                            onchange: on_apply_to_all_change,
                        }
                        span { class: "if-bulk-map__apply-label", "All modes ({modes.len()})" }
                    }
                }
            }

            div { role: "grid", class: "if-bulk-map__table",
                if has_axis_rows {
                    BulkMapRowsGroup {
                        title: "Axes".to_owned(),
                        rows: axis_rows,
                        target_vjoy: target_for_groups.clone(),
                        conflicting: axis_conflicting,
                        on_row_change: on_axis_change,
                        on_row_replace_toggle: on_axis_replace,
                        on_skip_all_conflicts: axis_chip_handlers.skip_all_conflicts,
                        on_replace_all_conflicts: axis_chip_handlers.replace_all_conflicts,
                        on_include_all: axis_chip_handlers.include_all,
                        on_exclude_all: axis_chip_handlers.exclude_all,
                    }
                }
                if has_button_rows {
                    BulkMapRowsGroup {
                        title: "Buttons".to_owned(),
                        rows: button_rows,
                        target_vjoy: target_for_groups.clone(),
                        conflicting: button_conflicting,
                        on_row_change: on_button_change,
                        on_row_replace_toggle: on_button_replace,
                        on_skip_all_conflicts: button_chip_handlers.skip_all_conflicts,
                        on_replace_all_conflicts: button_chip_handlers.replace_all_conflicts,
                        on_include_all: button_chip_handlers.include_all,
                        on_exclude_all: button_chip_handlers.exclude_all,
                    }
                }
                if has_hat_rows {
                    BulkMapRowsGroup {
                        title: "Hats".to_owned(),
                        rows: hat_rows,
                        target_vjoy: target_for_groups,
                        conflicting: hat_conflicting,
                        on_row_change: on_hat_change,
                        on_row_replace_toggle: on_hat_replace,
                        on_skip_all_conflicts: hat_chip_handlers.skip_all_conflicts,
                        on_replace_all_conflicts: hat_chip_handlers.replace_all_conflicts,
                        on_include_all: hat_chip_handlers.include_all,
                        on_exclude_all: hat_chip_handlers.exclude_all,
                    }
                }
                if !has_any_rows {
                    div { role: "row", class: "if-bulk-map__table-empty",
                        "No inputs available for this source device."
                    }
                }
            }
            footer { class: "if-bulk-map__footer",
                div { class: "if-bulk-map__footer-info",
                    div { class: "if-bulk-map__summary",
                        span { class: "if-bulk-map__summary-create", "+{counts.create} create" }
                        if *apply_to_all.read() {
                            span { class: "if-bulk-map__summary-modes", " across {modes.len()} modes" }
                        }
                        span { class: "if-bulk-map__summary-sep", " / " }
                        span { class: "if-bulk-map__summary-replace", "{counts.replace} replace" }
                        span { class: "if-bulk-map__summary-sep", " / " }
                        span { class: "if-bulk-map__summary-skip", "{counts.skip} skip" }
                        span { class: "if-bulk-map__summary-sep", " / " }
                        span { class: "if-bulk-map__summary-excluded", "{counts.excluded} excluded" }
                    }
                    span { class: "if-bulk-map__caption", "{snapshot_caption}" }
                }
                div { class: "if-bulk-map__footer-actions",
                    if is_dirty {
                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: on_reset,
                            "Reset"
                        }
                    }
                    Button { disabled: apply_count == 0, onclick: on_apply, "{apply_label}" }
                }
            }
        }
    }
}

#[component]
fn BulkMapRowsGroup(
    title: String,
    rows: Vec<RowState>,
    target_vjoy: Option<VirtualDeviceConfig>,
    conflicting: Vec<bool>,
    on_row_change: EventHandler<(u8, Option<OutputAddress>)>,
    on_row_replace_toggle: EventHandler<u8>,
    on_skip_all_conflicts: EventHandler<()>,
    on_replace_all_conflicts: EventHandler<()>,
    on_include_all: EventHandler<()>,
    on_exclude_all: EventHandler<()>,
) -> Element {
    let row_refs = rows.iter().collect::<Vec<_>>();
    let render_skip = show_skip_all_conflicts(&row_refs, &conflicting);
    let render_replace = show_replace_all_conflicts(&row_refs, &conflicting);
    let render_include = show_include_all(&row_refs);
    let render_exclude = show_exclude_all(&row_refs);

    rsx! {
        div { role: "rowgroup", class: "if-bulk-map__group",
            div { role: "row", class: "if-bulk-map__group-header",
                span { class: "if-bulk-map__group-title", "{title} ({rows.len()})" }
                if render_skip {
                    BulkMapGroupChip {
                        label: "Skip all conflicts".to_owned(),
                        on_click: on_skip_all_conflicts,
                    }
                }
                if render_replace {
                    BulkMapGroupChip {
                        label: "Replace all conflicts".to_owned(),
                        on_click: on_replace_all_conflicts,
                    }
                }
                if render_include {
                    BulkMapGroupChip {
                        label: "Include all".to_owned(),
                        on_click: on_include_all,
                    }
                }
                if render_exclude {
                    BulkMapGroupChip {
                        label: "Exclude all".to_owned(),
                        on_click: on_exclude_all,
                    }
                }
            }
            for (index, row) in rows.iter().cloned().enumerate() {
                {
                    let key = row_key(&row);
                    let is_conflicting = conflicting.get(index).copied().unwrap_or(false);
                    rsx! {
                        BulkMapRow {
                            key: "{key}",
                            row,
                            is_conflicting,
                            target_vjoy: target_vjoy.clone(),
                            on_change: on_row_change,
                            on_replace_toggle: on_row_replace_toggle,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BulkMapGroupChip(label: String, on_click: EventHandler<()>) -> Element {
    let onclick = move |_| on_click.call(());
    rsx! {
        button {
            r#type: "button",
            class: "if-bulk-map__chip",
            onclick,
            "{label}"
        }
    }
}

#[component]
fn BulkMapRow(
    row: RowState,
    is_conflicting: bool,
    target_vjoy: Option<VirtualDeviceConfig>,
    on_change: EventHandler<(u8, Option<OutputAddress>)>,
    on_replace_toggle: EventHandler<u8>,
) -> Element {
    let kind_letter = match row.kind {
        RowKind::Axis => "A",
        RowKind::Button => "B",
        RowKind::Hat => "H",
    };
    let row_class = match row.kind {
        RowKind::Axis => "if-bulk-map__row if-bulk-map__row--axis",
        RowKind::Button => "if-bulk-map__row if-bulk-map__row--button",
        RowKind::Hat => "if-bulk-map__row if-bulk-map__row--hat",
    };
    let source_cell_class = match row.kind {
        RowKind::Axis => "if-bulk-map__source-cell if-bulk-map__source-cell--axis",
        RowKind::Button => "if-bulk-map__source-cell if-bulk-map__source-cell--button",
        RowKind::Hat => "if-bulk-map__source-cell if-bulk-map__source-cell--hat",
    };
    let source_label = source_row_label(row.kind, row.source_index);
    let target_options = build_target_options(row.kind, target_vjoy.as_ref());
    let current = row
        .target
        .as_ref()
        .map_or_else(|| DO_NOT_MAP_VALUE.to_owned(), format_output_value);
    let mut select_value = use_signal(|| current.clone());
    if select_value.peek().as_str() != current.as_str() {
        select_value.set(current.clone());
    }
    let id_attr = format!("bulk-map-row-{kind_letter}-{}", row.source_index);
    let on_target_change = {
        let kind = row.kind;
        let target_vjoy = target_vjoy.clone();
        let source_index = row.source_index;
        move |evt: FormEvent| {
            let value = evt.value();
            select_value.set(value.clone());
            let parsed = parse_target_value(kind, &value, target_vjoy.as_ref());
            on_change.call((source_index, parsed));
        }
    };
    let select_ro: ReadSignal<String> = select_value.into();
    let onclick = move |_| on_replace_toggle.call(row.source_index);
    let show_replace = is_conflicting || row.replace;

    rsx! {
        div { role: "row", class: "{row_class}",
            span { role: "gridcell", class: "if-bulk-map__kind", "{kind_letter}" }
            span { role: "gridcell", class: "{source_cell_class}",
                span { class: "if-bulk-map__source", "{source_label}" }
                span { class: "if-bulk-map__live-cell",
                    RowReadout { kind: row.kind, address: row.input.clone() }
                }
            }
            span { role: "gridcell", class: "if-bulk-map__target",
                Select {
                    id: Some(id_attr),
                    value: select_ro,
                    onchange: on_target_change,
                    options: target_options,
                }
            }
            if show_replace {
                span { role: "gridcell", class: "if-bulk-map__action",
                    button {
                        r#type: "button",
                        class: if row.replace {
                            "if-bulk-map__chip if-bulk-map__chip--active"
                        } else {
                            "if-bulk-map__chip"
                        },
                        "aria-pressed": "{row.replace}",
                        onclick,
                        if row.replace { "Replacing" } else { "Replace" }
                    }
                }
            } else {
                span {
                    role: "gridcell",
                    class: "if-bulk-map__action if-bulk-map__action--empty",
                    "aria-hidden": "true",
                }
            }
        }
    }
}

const DO_NOT_MAP_VALUE: &str = "(do not map)";

fn row_key(row: &RowState) -> String {
    format!("{:?}:{:?}:{}", row.input, row.kind, row.source_index)
}

fn source_row_label(kind: RowKind, source_index: u8) -> String {
    let display_index = u16::from(source_index) + 1;
    match kind {
        RowKind::Axis => format!("Axis {display_index}"),
        RowKind::Button => format!("Button {display_index}"),
        RowKind::Hat => format!("Hat {display_index}"),
    }
}

fn row_change_handler(
    mut wizard: Signal<WizardState>,
    kind: RowKind,
) -> EventHandler<(u8, Option<OutputAddress>)> {
    EventHandler::new(move |(index, target): (u8, Option<OutputAddress>)| {
        if let Some(row) = wizard
            .write()
            .rows
            .iter_mut()
            .find(|row| row.kind == kind && row.source_index == index)
        {
            row.target = target;
        }
    })
}

fn row_replace_handler(mut wizard: Signal<WizardState>, kind: RowKind) -> EventHandler<u8> {
    EventHandler::new(move |index: u8| {
        if let Some(row) = wizard
            .write()
            .rows
            .iter_mut()
            .find(|row| row.kind == kind && row.source_index == index)
        {
            row.replace = !row.replace;
        }
    })
}

struct GroupChipHandlers {
    skip_all_conflicts: EventHandler<()>,
    replace_all_conflicts: EventHandler<()>,
    include_all: EventHandler<()>,
    exclude_all: EventHandler<()>,
}

fn group_chip_handlers(
    wizard: Signal<WizardState>,
    kind: RowKind,
    target: Option<VirtualDeviceConfig>,
    conflicting_indices: Vec<u8>,
) -> GroupChipHandlers {
    GroupChipHandlers {
        skip_all_conflicts: set_conflict_replace_handler(
            wizard,
            kind,
            conflicting_indices.clone(),
            false,
        ),
        replace_all_conflicts: set_conflict_replace_handler(
            wizard,
            kind,
            conflicting_indices,
            true,
        ),
        include_all: include_all_handler(wizard, kind, target),
        exclude_all: exclude_all_handler(wizard, kind),
    }
}

fn row_conflicts(
    rows: &[RowState],
    profile: &inputforge_core::profile::Profile,
    modes: &[String],
) -> Vec<bool> {
    rows.iter()
        .map(|row| {
            let is_conflicting = modes
                .iter()
                .any(|mode| conflicts::existing_name_for(profile, &row.input, mode).is_some());
            debug_assert_eq!(
                is_conflicting,
                !conflicts::conflicting_modes(profile, &row.input, modes).is_empty()
            );
            is_conflicting
        })
        .collect()
}

fn conflicting_indices(rows: &[RowState], conflicting: &[bool]) -> Vec<u8> {
    rows.iter()
        .zip(conflicting.iter())
        .filter_map(|(row, &is_conflicting)| is_conflicting.then_some(row.source_index))
        .collect()
}

fn set_conflict_replace_handler(
    mut wizard: Signal<WizardState>,
    kind: RowKind,
    conflicting_indices: Vec<u8>,
    replace: bool,
) -> EventHandler<()> {
    EventHandler::new(move |()| {
        for row in wizard
            .write()
            .rows
            .iter_mut()
            .filter(|row| row.kind == kind && conflicting_indices.contains(&row.source_index))
        {
            row.replace = replace;
        }
    })
}

fn include_all_handler(
    mut wizard: Signal<WizardState>,
    kind: RowKind,
    target: Option<VirtualDeviceConfig>,
) -> EventHandler<()> {
    EventHandler::new(move |()| {
        let Some(target) = target.as_ref() else {
            return;
        };
        for row in wizard
            .write()
            .rows
            .iter_mut()
            .filter(|row| row.kind == kind && row.target.is_none())
        {
            row.target = auto_target_for(row.kind, row.source_index, target);
        }
    })
}

fn exclude_all_handler(mut wizard: Signal<WizardState>, kind: RowKind) -> EventHandler<()> {
    EventHandler::new(move |()| {
        for row in wizard
            .write()
            .rows
            .iter_mut()
            .filter(|row| row.kind == kind)
        {
            row.target = None;
        }
    })
}

fn derive_rows(
    source_id: &DeviceId,
    axes_count: u8,
    button_count: u8,
    hat_count: u8,
    target: &VirtualDeviceConfig,
) -> Vec<RowState> {
    let mut rows = Vec::new();
    for index in 0..axes_count {
        rows.push(RowState {
            kind: RowKind::Axis,
            source_index: index,
            input: InputAddress::Bound {
                device: source_id.clone(),
                input: InputId::Axis { index },
            },
            target: auto_axis_target(target, usize::from(index)),
            replace: false,
        });
    }
    for index in 0..button_count {
        rows.push(RowState {
            kind: RowKind::Button,
            source_index: index,
            input: InputAddress::Bound {
                device: source_id.clone(),
                input: InputId::Button { index },
            },
            target: auto_button_target(target, usize::from(index)),
            replace: false,
        });
    }
    for index in 0..hat_count {
        rows.push(RowState {
            kind: RowKind::Hat,
            source_index: index,
            input: InputAddress::Bound {
                device: source_id.clone(),
                input: InputId::Hat { index },
            },
            target: auto_hat_target(target, usize::from(index)),
            replace: false,
        });
    }
    rows
}

fn derive_rows_for_selection(
    source_id: Option<&DeviceId>,
    connected_devices: &[DeviceState],
    target_id: Option<u8>,
    virtual_devices: &[VirtualDeviceConfig],
) -> Vec<RowState> {
    let Some(source_id) = source_id else {
        return Vec::new();
    };
    let source = connected_devices
        .iter()
        .find(|device| &device.info.id == source_id);
    let target =
        target_id.and_then(|id| virtual_devices.iter().find(|device| device.device_id == id));
    match (source, target) {
        (Some(source), Some(target)) => derive_rows(
            &source.info.id,
            source.info.axes,
            source.info.buttons,
            source.info.hats,
            target,
        ),
        _ => Vec::new(),
    }
}

fn reconcile_wizard_state(
    state: &mut WizardState,
    connected_devices: &[DeviceState],
    virtual_devices: &[VirtualDeviceConfig],
) -> bool {
    let source_id = state
        .source_device_id
        .as_ref()
        .filter(|id| {
            connected_devices
                .iter()
                .any(|device| &device.info.id == *id)
        })
        .cloned()
        .or_else(|| {
            connected_devices
                .first()
                .map(|device| device.info.id.clone())
        });
    let target_id = state
        .target_vjoy_id
        .filter(|id| virtual_devices.iter().any(|device| device.device_id == *id))
        .or_else(|| virtual_devices.first().map(|device| device.device_id));
    let rows = derive_rows_for_selection(
        source_id.as_ref(),
        connected_devices,
        target_id,
        virtual_devices,
    );
    let changed = state.source_device_id != source_id
        || state.target_vjoy_id != target_id
        || !rows_match_current_capabilities(&state.rows, &rows, target_id, virtual_devices);
    if changed {
        state.source_device_id = source_id;
        state.target_vjoy_id = target_id;
        state.rows = rows;
    }
    changed
}

fn rows_match_current_capabilities(
    current: &[RowState],
    derived: &[RowState],
    target_id: Option<u8>,
    virtual_devices: &[VirtualDeviceConfig],
) -> bool {
    current.len() == derived.len()
        && current
            .iter()
            .zip(derived.iter())
            .all(|(current, derived)| {
                current.kind == derived.kind
                    && current.source_index == derived.source_index
                    && current.input == derived.input
                    && current.target.as_ref().is_none_or(|target| {
                        target_is_available(target, target_id, virtual_devices)
                    })
            })
}

fn target_is_available(
    address: &OutputAddress,
    target_id: Option<u8>,
    virtual_devices: &[VirtualDeviceConfig],
) -> bool {
    let Some(target_id) = target_id else {
        return false;
    };
    let Some(target) = virtual_devices
        .iter()
        .find(|device| device.device_id == target_id)
    else {
        return false;
    };
    if address.device != target.device_id {
        return false;
    }
    match address.output {
        OutputId::Axis { id } => target.axes.contains(&id),
        OutputId::Button { id } => id > 0 && id <= target.button_count,
        OutputId::Hat { id } => id > 0 && id <= target.hat_count,
    }
}

fn rows_dirty(rows: &[RowState], baseline: &[RowState]) -> bool {
    rows != baseline
}

fn auto_target_for(
    kind: RowKind,
    source_index: u8,
    target: &VirtualDeviceConfig,
) -> Option<OutputAddress> {
    match kind {
        RowKind::Axis => auto_axis_target(target, usize::from(source_index)),
        RowKind::Button => auto_button_target(target, usize::from(source_index)),
        RowKind::Hat => auto_hat_target(target, usize::from(source_index)),
    }
}

fn build_target_options(kind: RowKind, target: Option<&VirtualDeviceConfig>) -> Vec<SelectOption> {
    let mut options = vec![SelectOption {
        value: DO_NOT_MAP_VALUE.to_owned(),
        label: DO_NOT_MAP_VALUE.to_owned(),
        disabled: false,
        class: None,
    }];
    let Some(target) = target else {
        return options;
    };

    match kind {
        RowKind::Axis => {
            for axis in &target.axes {
                options.push(SelectOption {
                    value: format!("axis:{axis:?}"),
                    label: format_axis_label(*axis),
                    disabled: false,
                    class: None,
                });
            }
        }
        RowKind::Button => {
            for id in 1..=target.button_count {
                options.push(SelectOption {
                    value: format!("button:{id}"),
                    label: format!("Button {id}"),
                    disabled: false,
                    class: None,
                });
            }
        }
        RowKind::Hat => {
            for id in 1..=target.hat_count {
                options.push(SelectOption {
                    value: format!("hat:{id}"),
                    label: format!("Hat {id}"),
                    disabled: false,
                    class: None,
                });
            }
        }
    }
    options
}

fn format_axis_label(axis: VJoyAxis) -> String {
    match axis {
        VJoyAxis::X => "X axis".to_owned(),
        VJoyAxis::Y => "Y axis".to_owned(),
        VJoyAxis::Z => "Z axis".to_owned(),
        VJoyAxis::Rx => "Rx axis".to_owned(),
        VJoyAxis::Ry => "Ry axis".to_owned(),
        VJoyAxis::Rz => "Rz axis".to_owned(),
        VJoyAxis::Slider0 => "Slider 0".to_owned(),
        VJoyAxis::Slider1 => "Slider 1".to_owned(),
    }
}

fn format_output_value(address: &OutputAddress) -> String {
    match address.output {
        OutputId::Axis { id } => format!("axis:{id:?}"),
        OutputId::Button { id } => format!("button:{id}"),
        OutputId::Hat { id } => format!("hat:{id}"),
    }
}

fn parse_target_value(
    kind: RowKind,
    value: &str,
    target: Option<&VirtualDeviceConfig>,
) -> Option<OutputAddress> {
    if value == DO_NOT_MAP_VALUE {
        return None;
    }
    let target = target?;
    let (prefix, rest) = value.split_once(':')?;
    match (kind, prefix) {
        (RowKind::Axis, "axis") => {
            let axis = parse_axis(rest)?;
            Some(OutputAddress {
                device: target.device_id,
                output: OutputId::Axis { id: axis },
            })
        }
        (RowKind::Button, "button") => {
            let id = rest.parse::<u8>().ok()?;
            Some(OutputAddress {
                device: target.device_id,
                output: OutputId::Button { id },
            })
        }
        (RowKind::Hat, "hat") => {
            let id = rest.parse::<u8>().ok()?;
            Some(OutputAddress {
                device: target.device_id,
                output: OutputId::Hat { id },
            })
        }
        _ => None,
    }
}

fn parse_axis(value: &str) -> Option<VJoyAxis> {
    match value {
        "X" => Some(VJoyAxis::X),
        "Y" => Some(VJoyAxis::Y),
        "Z" => Some(VJoyAxis::Z),
        "Rx" => Some(VJoyAxis::Rx),
        "Ry" => Some(VJoyAxis::Ry),
        "Rz" => Some(VJoyAxis::Rz),
        "Slider0" => Some(VJoyAxis::Slider0),
        "Slider1" => Some(VJoyAxis::Slider1),
        _ => None,
    }
}

#[cfg(test)]
fn apply_for_test(
    state: &inputforge_core::state::AppState,
    wizard: &WizardState,
    modes: &[String],
    command_tx: &std::sync::mpsc::Sender<EngineCommand>,
) {
    let profile = state.active_profile.as_ref().expect("profile loaded");
    let entries = apply::build_entries(profile, &wizard.rows, modes);
    // Mirror the production handler: resolve via the snapshot
    // accessor so the test exercises the same alias lookup path
    // (alias > hardware > id, with disconnected-but-remembered
    // devices covered through `device_registry`).
    let cfg = ConfigSnapshot::from_state(state, None);
    let source_display_name = wizard
        .source_device_id
        .as_ref()
        .map_or_else(|| "source".to_owned(), |id| cfg.device_display_name(id));
    let snapshot_label =
        apply::format_snapshot_label(&source_display_name, wizard.target_vjoy_id.unwrap_or(0));
    let _ = command_tx.send(EngineCommand::SetMappingsBulk {
        entries,
        snapshot_label,
    });
}

#[cfg(test)]
impl WizardState {
    fn with_seed_rows(rows: Vec<RowState>, mode: String) -> Self {
        let mut state = Self::empty(mode);
        state.rows = rows;
        state
    }
}
