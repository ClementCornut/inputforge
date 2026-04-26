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
    let variant_class = variant.class_for("if-icon-button");
    let size_class = size.class_for("if-icon-button");
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: every `ButtonVariant` must produce a matching `IconButton` class
    /// via the shared `class_for(prefix)` delegation. If a new variant is added
    /// to `ButtonVariant`, this test exercises the full match exhaustively so the
    /// CSS author knows to ship `.if-icon-button--<modifier>` alongside.
    #[test]
    fn icon_button_class_for_every_variant() {
        for v in [
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonVariant::Ghost,
            ButtonVariant::Danger,
        ] {
            let cls = v.class_for("if-icon-button");
            assert!(cls.starts_with("if-icon-button--"), "got: {cls:?}");
            assert!(!cls.contains(' '), "got: {cls:?}");
        }
    }

    #[test]
    fn icon_button_class_for_every_size() {
        for s in [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg] {
            let cls = s.class_for("if-icon-button");
            assert!(cls.starts_with("if-icon-button--"), "got: {cls:?}");
            assert!(!cls.contains(' '), "got: {cls:?}");
        }
    }
}
