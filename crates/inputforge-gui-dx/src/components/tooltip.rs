use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TooltipPlacement {
    Top,
    Bottom,
    Left,
    Right,
}

#[component]
pub fn Tooltip(
    content: String,
    #[props(default = TooltipPlacement::Top)] placement: TooltipPlacement,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let p = match placement {
        TooltipPlacement::Top => "if-tooltip--top",
        TooltipPlacement::Bottom => "if-tooltip--bottom",
        TooltipPlacement::Left => "if-tooltip--left",
        TooltipPlacement::Right => "if-tooltip--right",
    };
    let combined = merge_class("if-tooltip", p, class.as_deref());
    rsx! {
        span { class: "{combined}",
            {children}
            span { class: "if-tooltip__bubble", role: "tooltip", "{content}" }
        }
    }
}
