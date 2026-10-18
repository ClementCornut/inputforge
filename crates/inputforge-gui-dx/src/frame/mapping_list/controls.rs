//! Rail filter controls and app listener wiring.
use super::filter::DeviceChip;
use crate::components::{InputSize, TextInput, ToggleChip};
use dioxus::prelude::*;
use inputforge_core::types::DeviceId;
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(super) fn DeviceFilterRow(
    chips: Vec<DeviceChip>,
    selected: Signal<Option<DeviceId>>,
    events: crate::navigation::Listener,
) -> Element {
    if chips.is_empty() {
        return rsx! {};
    }
    rsx! {
        div {
            class: "if-rail__device-filter",
            role: "group",
            "aria-label": "Filter mappings by device",
            for chip in chips {
                {
                    let active = selected.read().as_ref() == Some(&chip.id);
                    let id = chip.id.clone();
                    let label = chip.label.clone();
                    rsx! {
                        ToggleChip {
                            class: "if-rail__device-chip",
                            pressed: active,
                            title: label.clone(),
                            aria_label: format!("Filter mappings by {label}"),
                            onclick: move |_| events.device.call(Some(id.clone())),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(super) fn FilterInput(
    value: Signal<String>,
    focused: Signal<bool>,
    events: crate::navigation::Listener,
) -> Element {
    let mut focused = focused;
    rsx! {
        div {
            class: "if-rail__filter",
            onfocusin: move |_| focused.set(true),
            onfocusout: move |_| focused.set(false),
            TextInput {
                aria_label: "Filter mappings",
                id: "mapping-filter",
                value: ReadSignal::from(value),
                size: InputSize::Sm,
                placeholder: "Filter mappings\u{2026}".to_owned(),
                oninput: move |evt: FormEvent| events.query.call(evt.value()),
            }
        }
    }
}
