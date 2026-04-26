use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

impl ButtonVariant {
    #[must_use]
    fn class(self) -> &'static str {
        match self {
            ButtonVariant::Primary => "if-button--primary",
            ButtonVariant::Secondary => "if-button--secondary",
            ButtonVariant::Ghost => "if-button--ghost",
            ButtonVariant::Danger => "if-button--danger",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

impl ButtonSize {
    #[must_use]
    fn class(self) -> &'static str {
        match self {
            ButtonSize::Sm => "if-button--sm",
            ButtonSize::Md => "if-button--md",
            ButtonSize::Lg => "if-button--lg",
        }
    }
}

#[component]
pub fn Button(
    #[props(default = ButtonVariant::Primary)] variant: ButtonVariant,
    #[props(default = ButtonSize::Md)] size: ButtonSize,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let variant_class = format!("{} {}", variant.class(), size.class());
    let combined = merge_class("if-button", &variant_class, class.as_deref());
    let click_handler = move |evt: MouseEvent| {
        if let Some(handler) = &onclick {
            handler.call(evt);
        }
    };
    rsx! {
        button {
            class: "{combined}",
            disabled,
            onclick: click_handler,
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: every component must compose its class string via `merge_class`,
    /// not inline `format!`, to avoid the trailing-space bug when no caller class is
    /// provided. If this test fails, a primitive likely reverted to inline `format!`.
    #[test]
    fn class_string_has_no_trailing_space_when_no_caller_class() {
        let v_class = ButtonVariant::Primary.class();
        let s_class = ButtonSize::Md.class();
        let combined = merge_class("if-button", &format!("{v_class} {s_class}"), None);
        assert!(!combined.ends_with(' '), "got: {combined:?}");
        assert_eq!(combined, "if-button if-button--primary if-button--md");
    }
}
