//! F-bulk-map: side-panel bulk mapping wizard. See
//! `docs/superpowers/specs/2026-05-03-bulk-mapping-design.md`.

#![allow(
    dead_code,
    reason = "Module wired progressively across tasks 9 to 18; allow removed in 18d."
)]

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
use inputforge_core::types::{DeviceId, VirtualDeviceConfig};

use crate::components::{Button, Checkbox, Field, Select};
use crate::context::AppContext;
use crate::frame::bulk_map::empty_state::NoVjoyEmptyState;
use crate::frame::bulk_map::state::WizardState;
use crate::frame::view_state::{PanelSlot, ViewState};

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

    let on_source_change = move |evt: FormEvent| {
        let value = evt.value();
        source_value.set(value.clone());
        wizard.write().source_device_id = Some(DeviceId(value));
    };
    let on_target_change = move |evt: FormEvent| {
        let value = evt.value();
        target_value.set(value.clone());
        if let Ok(id) = value.parse::<u8>() {
            wizard.write().target_vjoy_id = Some(id);
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

            div { class: "if-bulk-map__table" }
            div { class: "if-bulk-map__summary" }
            footer { class: "if-bulk-map__footer",
                Button { onclick: move |_| panel.set(PanelSlot::None), "Cancel" }
                Button { disabled: true, onclick: move |_| {}, "Apply" }
            }
            div { class: "if-bulk-map__caption", "{snapshot_caption}" }
        }
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
