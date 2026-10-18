//! Reusable empty-state content; the consumer owns screen sizing and copy.

use dioxus::prelude::*;

use crate::components::merge_class;

/// Empty-state heading, supporting copy, optional readout and action slots.
#[component]
pub fn EmptyState(
    title: String,
    #[props(default)] description: Option<String>,
    #[props(default)] eyebrow: Option<Element>,
    #[props(default)] actions: Option<Element>,
    #[props(default)] class: Option<String>,
    #[props(default)] children: Element,
) -> Element {
    let combined = merge_class("if-empty", "", class.as_deref());
    rsx! {
        div { class: combined,
            if let Some(eyebrow) = eyebrow { {eyebrow} }
            h2 { class: "if-empty__heading", "{title}" }
            if let Some(description) = description {
                p { class: "if-empty__description", "{description}" }
            }
            {children}
            if let Some(actions) = actions {
                div { class: "if-empty__actions", {actions} }
            }
        }
    }
}
