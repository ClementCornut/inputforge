use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeparatorOrientation {
    Horizontal,
    Vertical,
}

#[component]
pub fn Separator(
    #[props(default = SeparatorOrientation::Horizontal)] orientation: SeparatorOrientation,
    #[props(default)] class: Option<String>,
) -> Element {
    let v = match orientation {
        SeparatorOrientation::Horizontal => "if-separator--horizontal",
        SeparatorOrientation::Vertical => "if-separator--vertical",
    };
    let combined = merge_class("if-separator", v, class.as_deref());
    rsx! { div { class: "{combined}", role: "separator" } }
}
