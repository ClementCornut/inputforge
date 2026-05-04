//! F-bulk-map: side-panel bulk mapping wizard. See
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
use inputforge_core::types::{
    DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis, VirtualDeviceConfig,
};

use crate::components::{Button, Checkbox, Field, Select};
use crate::context::AppContext;
use crate::frame::bulk_map::auto_map::{auto_axis_target, auto_button_target, auto_hat_target};
use crate::frame::bulk_map::empty_state::NoVjoyEmptyState;
use crate::frame::bulk_map::group_actions::{
    show_exclude_all, show_include_all, show_replace_all_conflicts, show_skip_all_conflicts,
};
use crate::frame::bulk_map::row_readout::RowReadout;
use crate::frame::bulk_map::state::{RowKind, RowState, WizardState};
use crate::frame::view_state::{PanelSlot, ViewState};
use crate::toast::{ToastLevel, ToastQueue};

const BULK_MAP_CSS: Asset = asset!("/assets/frame/bulk_map.css");

#[component]
pub(crate) fn BulkMapPanel() -> Element {
    tracing::trace!(target: "frame::render", region = "bulk_map");

    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let mut panel = view.panel_slot;

    let virtual_devices: Vec<VirtualDeviceConfig> = ctx.state.read().virtual_devices.clone();
    let has_profile = ctx.state.read().active_profile.is_some();

    if !has_profile {
        return rsx! {
            Stylesheet { href: BULK_MAP_CSS }
            section { class: "if-bulk-map", "aria-label": "Bulk-map device wizard",
                BulkMapHeader { on_close: move |_| panel.set(PanelSlot::None) }
                NoVjoyEmptyState {
                    title: "No profile loaded".to_owned(),
                    caption: "Load or create a profile, then reopen.".to_owned(),
                }
                footer { class: "if-bulk-map__footer",
                    Button { onclick: move |_| panel.set(PanelSlot::None), "Cancel" }
                    Button { disabled: true, onclick: move |_| {}, "Apply" }
                }
            }
        };
    }

    if virtual_devices.is_empty() {
        return rsx! {
            Stylesheet { href: BULK_MAP_CSS }
            section { class: "if-bulk-map", "aria-label": "Bulk-map device wizard",
                BulkMapHeader { on_close: move |_| panel.set(PanelSlot::None) }
                NoVjoyEmptyState {}
                footer { class: "if-bulk-map__footer",
                    Button { onclick: move |_| panel.set(PanelSlot::None), "Cancel" }
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
    let mut panel = view.panel_slot;

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
        if let (Some(source), Some(target)) = (connected_devices.first(), virtual_devices.first()) {
            state.rows = derive_rows(
                &source.info.id,
                source.info.axes,
                source.info.buttons,
                source.info.hats,
                target,
            );
        }
        state
    });

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

    let on_source_change = {
        let connected_devices = connected_devices.clone();
        let virtual_devices = virtual_devices.clone();
        move |evt: FormEvent| {
            let value = evt.value();
            source_value.set(value.clone());
            let source_id = DeviceId(value);
            let mut state = wizard.write();
            state.source_device_id = Some(source_id.clone());
            let target = state
                .target_vjoy_id
                .and_then(|id| virtual_devices.iter().find(|device| device.device_id == id));
            let source = connected_devices
                .iter()
                .find(|device| device.info.id == source_id);
            state.rows = match (source, target) {
                (Some(source), Some(target)) => derive_rows(
                    &source.info.id,
                    source.info.axes,
                    source.info.buttons,
                    source.info.hats,
                    target,
                ),
                _ => Vec::new(),
            };
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
                let source = state.source_device_id.as_ref().and_then(|source_id| {
                    connected_devices
                        .iter()
                        .find(|device| &device.info.id == source_id)
                });
                let target = virtual_devices.iter().find(|device| device.device_id == id);
                state.rows = match (source, target) {
                    (Some(source), Some(target)) => derive_rows(
                        &source.info.id,
                        source.info.axes,
                        source.info.buttons,
                        source.info.hats,
                        target,
                    ),
                    _ => Vec::new(),
                };
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

    let source_options = connected_devices
        .iter()
        .map(|device| (device.info.id.0.clone(), device.info.name.clone()))
        .collect::<Vec<_>>();
    let target_options = virtual_devices
        .iter()
        .map(|device| {
            (
                device.device_id.to_string(),
                format!(
                    "vJoy {}: {} axes, {} buttons, {} hat{}",
                    device.device_id,
                    device.axes.len(),
                    device.button_count,
                    device.hat_count,
                    if device.hat_count == 1 { "" } else { "s" },
                ),
            )
        })
        .collect::<Vec<_>>();
    let mode_options = modes
        .iter()
        .map(|mode| (mode.clone(), mode.clone()))
        .collect::<Vec<_>>();
    let snapshot_caption = "Snapshot taken before apply.";
    let target_for_groups = wizard.peek().target_vjoy_id.and_then(|id| {
        virtual_devices
            .iter()
            .find(|device| device.device_id == id)
            .cloned()
    });
    let rows = wizard.read().rows.clone();
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
    let current_mode = wizard.peek().mode.clone();
    let (axis_conflicting, button_conflicting, hat_conflicting) = {
        let state = ctx.state.read();
        if let Some(profile) = state.active_profile.as_ref() {
            (
                row_conflicts(&axis_rows, profile, &current_mode),
                row_conflicts(&button_rows, profile, &current_mode),
                row_conflicts(&hat_rows, profile, &current_mode),
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
    let active_modes = if *apply_to_all.read() {
        modes.clone()
    } else {
        vec![wizard.read().mode.clone()]
    };
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
    let on_apply = {
        let cmd_tx = ctx.commands.clone();
        let ctx = ctx.clone();
        let mut panel = panel;
        let active_modes = active_modes.clone();
        move |_| {
            let state = wizard.peek().clone();
            let entries = {
                let app_state = ctx.state.read();
                let profile = app_state.active_profile.as_ref().expect("profile loaded");
                apply::build_entries(profile, &state.rows, &active_modes)
            };
            let count = entries.len();
            let snapshot_label = apply::format_snapshot_label(
                state
                    .source_device_id
                    .as_ref()
                    .map_or("source", |device| device.0.as_str()),
                state.target_vjoy_id.unwrap_or(0),
            );
            let _ = cmd_tx.send(EngineCommand::SetMappingsBulk {
                entries,
                snapshot_label,
            });
            toast.push(ToastLevel::Success, format!("Created {count} mappings"));
            panel.set(PanelSlot::None);
        }
    };

    rsx! {
        section { class: "if-bulk-map", "aria-label": "Bulk-map device wizard",
            BulkMapHeader { on_close: move |_| panel.set(PanelSlot::None) }

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
                    label: format!("Apply to all modes ({})", modes.len()),
                    for_id: Some("bulk-map-all-modes".to_owned()),
                    Checkbox {
                        id: Some("bulk-map-all-modes".to_owned()),
                        checked: apply_to_all_ro,
                        onchange: on_apply_to_all_change,
                    }
                }
            }

            div { role: "grid", class: "if-bulk-map__table",
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
            footer { class: "if-bulk-map__footer",
                Button { onclick: move |_| panel.set(PanelSlot::None), "Cancel" }
                Button { disabled: apply_count == 0, onclick: on_apply, "{apply_label}" }
            }
            div { class: "if-bulk-map__caption", "{snapshot_caption}" }
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
                        label: "skip all conflicts".to_owned(),
                        on_click: on_skip_all_conflicts,
                    }
                }
                if render_replace {
                    BulkMapGroupChip {
                        label: "replace all conflicts".to_owned(),
                        on_click: on_replace_all_conflicts,
                    }
                }
                if render_include {
                    BulkMapGroupChip {
                        label: "include all".to_owned(),
                        on_click: on_include_all,
                    }
                }
                if render_exclude {
                    BulkMapGroupChip {
                        label: "exclude all".to_owned(),
                        on_click: on_exclude_all,
                    }
                }
            }
            for row in rows.iter().cloned() {
                BulkMapRow {
                    row,
                    target_vjoy: target_vjoy.clone(),
                    on_change: on_row_change,
                    on_replace_toggle: on_row_replace_toggle,
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
    target_vjoy: Option<VirtualDeviceConfig>,
    on_change: EventHandler<(u8, Option<OutputAddress>)>,
    on_replace_toggle: EventHandler<u8>,
) -> Element {
    let kind_letter = match row.kind {
        RowKind::Axis => "A",
        RowKind::Button => "B",
        RowKind::Hat => "H",
    };
    let source_label = match row.kind {
        RowKind::Axis => format!("Axis {}", row.source_index),
        RowKind::Button => format!("Btn {}", row.source_index),
        RowKind::Hat => format!("Hat {}", row.source_index),
    };
    let target_options = build_target_options(row.kind, target_vjoy.as_ref());
    let current = row
        .target
        .as_ref()
        .map_or_else(|| DO_NOT_MAP_VALUE.to_owned(), format_output_value);
    let mut select_value = use_signal(|| current);
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

    rsx! {
        div { role: "row", class: "if-bulk-map__row",
            span { role: "gridcell", class: "if-bulk-map__kind", "{kind_letter}" }
            span { role: "gridcell", class: "if-bulk-map__source", "{source_label}" }
            span { role: "gridcell", class: "if-bulk-map__live-cell",
                RowReadout { kind: row.kind, address: row.input.clone() }
            }
            span { role: "gridcell", class: "if-bulk-map__target",
                Select {
                    id: Some(id_attr),
                    value: select_ro,
                    onchange: on_target_change,
                    options: target_options,
                }
            }
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
                    if row.replace { "replacing" } else { "replace" }
                }
            }
        }
    }
}

const DO_NOT_MAP_VALUE: &str = "(do not map)";

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
    mode: &str,
) -> Vec<bool> {
    rows.iter()
        .map(|row| {
            let modes = [mode.to_owned()];
            let is_conflicting = conflicts::existing_name_for(profile, &row.input, mode).is_some();
            debug_assert_eq!(
                is_conflicting,
                !conflicts::conflicting_modes(profile, &row.input, &modes).is_empty()
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

fn build_target_options(
    kind: RowKind,
    target: Option<&VirtualDeviceConfig>,
) -> Vec<(String, String)> {
    let mut options = vec![(DO_NOT_MAP_VALUE.to_owned(), DO_NOT_MAP_VALUE.to_owned())];
    let Some(target) = target else {
        return options;
    };

    match kind {
        RowKind::Axis => {
            for axis in &target.axes {
                options.push((format!("axis:{axis:?}"), format_axis_label(*axis)));
            }
        }
        RowKind::Button => {
            for id in 1..=target.button_count {
                options.push((format!("button:{id}"), format!("Button {id}")));
            }
        }
        RowKind::Hat => {
            for id in 1..=target.hat_count {
                options.push((format!("hat:{id}"), format!("Hat {id}")));
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
    let snapshot_label = apply::format_snapshot_label(
        wizard
            .source_device_id
            .as_ref()
            .map_or("source", |device| device.0.as_str()),
        wizard.target_vjoy_id.unwrap_or(0),
    );
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

#[component]
fn BulkMapHeader(on_close: EventHandler<MouseEvent>) -> Element {
    let onclick = move |evt| on_close.call(evt);
    rsx! {
        header { class: "if-bulk-map__header",
            h2 { class: "if-bulk-map__title", "Bulk-map device" }
            button {
                r#type: "button",
                class: "if-bulk-map__close",
                "aria-label": "Close panel",
                title: "Esc",
                onclick,
                "x"
            }
        }
    }
}
