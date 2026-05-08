//! Empty-state renderers for the F8 mapping list rail.
//!
//! State A, zero mappings overall (profile loaded, mode has none):
//!   title + helper. The dashed `+ Add mapping` footer in the rail
//!   itself is the only affordance to start a mapping.
//!
//! State B, zero filter results: title naming active filters + helper +
//!   independent ghost clear buttons.

use dioxus::prelude::*;

use crate::components::{Button, ButtonSize, ButtonVariant};

#[component]
pub(crate) fn EmptyZeroMappings() -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::empty_zero_mappings");
    rsx! {
        div { class: "if-rail-empty if-rail-empty--zero-mappings",
            div { class: "if-rail-empty__title", "No mappings yet" }
            div { class: "if-rail-empty__helper",
                "Click + Add mapping below to start one."
            }
        }
    }
}

#[component]
pub(crate) fn EmptyZeroFilterResults(
    query: String,
    device_label: Option<String>,
    on_clear_text: EventHandler<()>,
    on_clear_device: Option<EventHandler<()>>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::empty_zero_filter_results");
    let has_query = !query.trim().is_empty();
    rsx! {
        div { class: "if-rail-empty if-rail-empty--zero-filter-results",
            div { class: "if-rail-empty__title",
                "No mappings match"
                if has_query {
                    " "
                    span { class: "muted", "\"{query}\"" }
                }
                if let Some(label) = &device_label {
                    " on "
                    span { class: "muted", "{label}" }
                }
            }
            div { class: "if-rail-empty__helper",
                "Filter searches name and source label."
            }
            div { class: "if-rail-empty__actions",
                if has_query {
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        onclick: move |_| on_clear_text.call(()),
                        "Clear text"
                    }
                }
                if let Some(clear_device) = on_clear_device {
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        onclick: move |_| clear_device.call(()),
                        "Clear device"
                    }
                }
            }
        }
    }
}
