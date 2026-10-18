//! Browser-owned disclosure behavior with shared focus presentation.

use dioxus::prelude::*;

use crate::components::merge_class;

/// Native collapsed disclosure. The browser owns toggling and keyboard behavior.
#[component]
pub fn Disclosure(
    summary: Element,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-disclosure", "", class.as_deref());
    rsx! {
        details { class: combined,
            summary { class: "if-disclosure__summary", {summary} }
            {children}
        }
    }
}
