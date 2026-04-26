use dioxus::prelude::*;

use super::merge_class;
use crate::components::Icon;
use crate::components::button::{ButtonSize, ButtonVariant};
use crate::icons::Icon as IconKind;

#[component]
pub fn IconButton(
    icon: IconKind,
    label: &'static str,
    #[props(default = ButtonVariant::Ghost)] variant: ButtonVariant,
    #[props(default = ButtonSize::Md)] size: ButtonSize,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    let variant_class = match variant {
        ButtonVariant::Primary => "if-icon-button--primary",
        ButtonVariant::Secondary => "if-icon-button--secondary",
        ButtonVariant::Ghost => "if-icon-button--ghost",
        ButtonVariant::Danger => "if-icon-button--danger",
    };
    let size_class = match size {
        ButtonSize::Sm => "if-icon-button--sm",
        ButtonSize::Md => "if-icon-button--md",
        ButtonSize::Lg => "if-icon-button--lg",
    };
    let combined = merge_class(
        "if-icon-button",
        &format!("{variant_class} {size_class}"),
        class.as_deref(),
    );
    let click_handler = move |evt: MouseEvent| {
        if let Some(handler) = &onclick {
            handler.call(evt);
        }
    };
    rsx! {
        button {
            class: "{combined}",
            "aria-label": label,
            disabled,
            onclick: click_handler,
            Icon { name: icon }
        }
    }
}
