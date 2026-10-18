use dioxus::prelude::*;

use crate::components::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeparatorOrientation {
    Horizontal,
    Vertical,
}

#[component]
pub fn Separator(
    #[props(default = SeparatorOrientation::Horizontal)] orientation: SeparatorOrientation,
    #[props(default)] decorative: bool,
    #[props(default)] strong: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let v = match orientation {
        SeparatorOrientation::Horizontal => "if-separator--horizontal",
        SeparatorOrientation::Vertical => "if-separator--vertical",
    };
    let variants = if strong {
        format!("{v} if-separator--strong")
    } else {
        v.to_owned()
    };
    let combined = merge_class("if-separator", &variants, class.as_deref());
    let aria_orientation = match orientation {
        SeparatorOrientation::Horizontal => "horizontal",
        SeparatorOrientation::Vertical => "vertical",
    };
    rsx! { div {
        class: combined,
        role: (!decorative).then_some("separator"),
        "aria-hidden": decorative.then_some("true"),
        "aria-orientation": (!decorative).then_some(aria_orientation),
    } }
}
