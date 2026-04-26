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
    let combined = merge_class("if-icon", size.class(), class.as_deref());
    rsx! {
        span {
            class: "{combined}",
            dangerous_inner_html: name.svg(),
        }
    }
}

/// Joins our default class, the variant class, and an optional caller class.
/// Pure-Rust, exported for re-use by other primitives. Skips empty parts so
/// callers may pass `""` as the variant (used by primitives without size/variant
/// modifiers like Slider, Label, Field) without producing double spaces.
pub(crate) fn merge_class(base: &str, variant: &str, caller: Option<&str>) -> String {
    let mut out = String::from(base);
    if !variant.is_empty() {
        out.push(' ');
        out.push_str(variant);
    }
    if let Some(c) = caller {
        if !c.is_empty() {
            out.push(' ');
            out.push_str(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::merge_class;

    #[test]
    fn merge_with_caller() {
        assert_eq!(
            merge_class("if-icon", "if-icon--md", Some("custom")),
            "if-icon if-icon--md custom"
        );
    }

    #[test]
    fn merge_without_caller() {
        assert_eq!(
            merge_class("if-icon", "if-icon--md", None),
            "if-icon if-icon--md"
        );
    }

    #[test]
    fn merge_with_empty_caller() {
        assert_eq!(
            merge_class("if-icon", "if-icon--md", Some("")),
            "if-icon if-icon--md"
        );
    }
}
