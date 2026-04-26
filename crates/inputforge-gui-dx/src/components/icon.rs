//! Renders an SVG icon. SVG content is trusted (Phosphor upstream),
//! injected via `dangerous_inner_html` — never feed user-provided
//! SVG through this component.

use dioxus::prelude::*;

use crate::icons::{Icon as IconKind, IconSize};

#[component]
pub fn Icon(
    name: IconKind,
    #[props(default = IconSize::Md)] size: IconSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = super::merge_class("if-icon", size.class(), class.as_deref());
    rsx! {
        span {
            class: "{combined}",
            dangerous_inner_html: name.svg(),
        }
    }
}
