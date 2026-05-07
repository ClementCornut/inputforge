use dioxus::prelude::*;

use crate::components::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Icon};
use crate::icons::Icon as IconKind;

#[component]
pub fn BottomDrawer(
    open: bool,
    title: String,
    count: usize,
    on_toggle: EventHandler<MouseEvent>,
    actions: Element,
    children: Element,
) -> Element {
    let icon = if open {
        IconKind::ChevronDown
    } else {
        IconKind::ChevronUp
    };

    rsx! {
        section {
            class: if open { "if-bottom-drawer if-bottom-drawer--open" } else { "if-bottom-drawer" },
            div { class: "if-bottom-drawer__header",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "if-bottom-drawer__toggle".to_owned(),
                    onclick: move |evt| on_toggle.call(evt),
                    Icon { name: icon }
                    span { class: "if-bottom-drawer__title", "{title}" }
                    Badge { variant: BadgeVariant::Neutral, "{count}" }
                }
                div { class: "if-bottom-drawer__actions", {actions} }
            }
            if open {
                div { class: "if-bottom-drawer__body", {children} }
            }
        }
    }
}
