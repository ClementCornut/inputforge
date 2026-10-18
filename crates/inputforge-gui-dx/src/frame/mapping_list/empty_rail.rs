//! Empty rail composition keeps capture and filter actions mounted.
use super::{
    MAPPING_LIST_CSS,
    add_inline::AddInline,
    controls::{DeviceFilterRow, FilterInput},
    empty::{EmptyZeroFilterResults, EmptyZeroMappings},
    filter::DeviceChip,
};
use crate::mapping_rail::Target;
use dioxus::prelude::*;
#[component]
pub(super) fn EmptyRail(
    total: usize,
    keyboard: crate::navigation::Listener,
    focused: Signal<bool>,
    force_expanded: Signal<bool>,
    pending_duplicate: Signal<Option<Target>>,
    chips: Vec<DeviceChip>,
    navigation_context: String,
) -> Element {
    let publication = use_context::<crate::mapping_edits::ui::Rail>();
    let filter = publication.filter;
    let selected_device_label = filter.device.read().as_ref().and_then(|selected| {
        chips
            .iter()
            .find(|chip| &chip.id == selected)
            .map(|chip| chip.label.clone())
    });
    let device_selected = filter.device.read().is_some();
    let query = filter.query.read().clone();
    rsx! {
        Stylesheet { href: MAPPING_LIST_CSS }
        if let Some(error) = publication.error.read().as_ref() {
            crate::components::InlineFeedback {
                tone: crate::components::FeedbackTone::Error,
                alert: true,
                "{error}"
            }
        }
        div {
            class: "if-rail",
            "data-navigation-context": navigation_context,
            if total == 0 {
                EmptyZeroMappings {}
            } else {
                FilterInput { value: filter.query, focused, events: keyboard }
                DeviceFilterRow { chips, selected: filter.device, events: keyboard }
                EmptyZeroFilterResults {
                    query,
                    device_label: selected_device_label,
                    on_clear_text: move |()| keyboard.query.call(String::new()),
                    on_clear_device: device_selected.then(|| EventHandler::new(move |()| keyboard.device.call(None))),
                }
            }
            div { class: "if-rail__add-sticky",
                AddInline { force_expanded, pending_duplicate }
            }
        }
    }
}
