use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeVariant {
    Neutral,
    Info,
    Success,
    Warning,
    Error,
}

#[component]
pub fn Badge(
    #[props(default = BadgeVariant::Neutral)] variant: BadgeVariant,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let v = match variant {
        BadgeVariant::Neutral => "if-badge--neutral",
        BadgeVariant::Info => "if-badge--info",
        BadgeVariant::Success => "if-badge--success",
        BadgeVariant::Warning => "if-badge--warning",
        BadgeVariant::Error => "if-badge--error",
    };
    let combined = merge_class("if-badge", v, class.as_deref());
    rsx! { span { class: "{combined}", {children} } }
}
