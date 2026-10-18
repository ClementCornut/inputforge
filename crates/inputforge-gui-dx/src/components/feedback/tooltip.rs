use dioxus::prelude::*;

use crate::components::merge_class;

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
    /// Bubble ID. Set the trigger's `aria_describedby` to this same stable ID.
    /// The wrapper never mutates children or invents a trigger association.
    #[props(default)]
    id: Option<String>,
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
            span { id, class: "if-tooltip__bubble", role: "tooltip", "{content}" }
        }
    }
}
