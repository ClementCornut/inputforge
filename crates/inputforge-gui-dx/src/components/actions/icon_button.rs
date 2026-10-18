//! Native icon-only actions with contextual owned accessible names.
use super::types::{ButtonSize, ButtonVariant};
use crate::components::{Icon, merge_class};
use crate::icons::Icon as IconKind;
use dioxus::prelude::*;

/// Render one native button containing a decorative icon.
#[component]
pub fn IconButton(
    icon: IconKind,
    label: String,
    #[props(default = ButtonVariant::Ghost)] variant: ButtonVariant,
    #[props(default = ButtonSize::Md)] size: ButtonSize,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    #[props(default = "button".to_owned())] button_type: String,
    #[props(default)] id: Option<String>,
    #[props(default)] title: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] aria_pressed: Option<bool>,
    #[props(default)] aria_expanded: Option<bool>,
    #[props(default)] aria_controls: Option<String>,
    #[props(default)] aria_haspopup: Option<String>,
    #[props(default)] autofocus: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] oncontextmenu: Option<EventHandler<MouseEvent>>,
    #[props(default)] onmounted: Option<EventHandler<MountedEvent>>,
) -> Element {
    let variant_class = variant.class_for("if-icon-button");
    let size_class = size.class_for("if-icon-button");
    let combined = merge_class(
        "if-icon-button",
        &format!("{variant_class} {size_class}"),
        class.as_deref(),
    );
    let onclick = move |event| {
        if !disabled && let Some(handler) = onclick {
            handler.call(event);
        }
    };
    let onkeydown = move |event| {
        if !disabled && let Some(handler) = onkeydown {
            handler.call(event);
        }
    };
    let oncontextmenu = move |event| {
        if !disabled && let Some(handler) = oncontextmenu {
            handler.call(event);
        }
    };
    let onmounted = move |event| {
        if let Some(handler) = onmounted {
            handler.call(event);
        }
    };
    rsx! {
        button {
            r#type: button_type,
            id,
            title,
            class: combined,
            "aria-label": label,
            "aria-labelledby": aria_labelledby,
            "aria-describedby": aria_describedby,
            "aria-pressed": aria_pressed.map(|value| value.to_string()),
            "aria-expanded": aria_expanded.map(|value| value.to_string()),
            "aria-controls": aria_controls,
            "aria-haspopup": aria_haspopup,
            disabled,
            autofocus,
            onclick,
            onkeydown,
            oncontextmenu,
            onmounted,
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
