use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardPadding {
    Sm,
    Md,
    Lg,
}

#[component]
pub fn Card(
    #[props(default = CardPadding::Md)] padding: CardPadding,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let pad_class = match padding {
        CardPadding::Sm => "if-card--pad-sm",
        CardPadding::Md => "if-card--pad-md",
        CardPadding::Lg => "if-card--pad-lg",
    };
    let combined = merge_class("if-card", pad_class, class.as_deref());
    rsx! { div { class: "{combined}", {children} } }
}
