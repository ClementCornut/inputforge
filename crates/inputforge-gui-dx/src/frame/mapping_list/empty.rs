//! Empty-state renderers for the F8 mapping list rail.
//!
//! State A — zero mappings overall (profile loaded, mode has none):
//!   title + helper + primary `+ Add mapping` button that expands directly
//!   into `CapturingArmed` (skips Resting -> click).
//!
//! State B — zero filter results: title quoting `<query>` + helper +
//!   ghost-link `Clear filter` button.

use dioxus::prelude::*;

use crate::components::{Button, ButtonVariant};

#[component]
pub(crate) fn EmptyZeroMappings(on_start_capture: EventHandler<()>) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::empty_zero_mappings");
    rsx! {
        div { class: "if-rail-empty if-rail-empty--zero-mappings",
            div { class: "if-rail-empty__title", "No mappings yet" }
            div { class: "if-rail-empty__helper",
                "Press an input on any connected device, or name a mapping below."
            }
            Button {
                variant: ButtonVariant::Primary,
                onclick: move |_| on_start_capture.call(()),
                "+ Add mapping"
            }
        }
    }
}

#[component]
pub(crate) fn EmptyZeroFilterResults(query: String, on_clear: EventHandler<()>) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::empty_zero_filter_results");
    rsx! {
        div { class: "if-rail-empty if-rail-empty--zero-filter-results",
            div { class: "if-rail-empty__title",
                "No mappings match "
                span { class: "muted", "\"{query}\"" }
            }
            div { class: "if-rail-empty__helper",
                "Filter searches name and source label."
            }
            Button {
                variant: ButtonVariant::Ghost,
                onclick: move |_| on_clear.call(()),
                "Clear filter"
            }
        }
    }
}
