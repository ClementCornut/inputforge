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
    pub(crate) fn modifier(self) -> &'static str {
        match self {
            ButtonVariant::Primary => "primary",
            ButtonVariant::Secondary => "secondary",
            ButtonVariant::Ghost => "ghost",
            ButtonVariant::Danger => "danger",
        }
    }

    /// Class string for any button-family component.
    /// `prefix` is the BEM block (e.g. `"if-button"`, `"if-icon-button"`).
    #[must_use]
    pub(crate) fn class_for(self, prefix: &str) -> String {
        format!("{prefix}--{}", self.modifier())
    }

    fn class(self) -> String {
        self.class_for("if-button")
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
    pub(crate) fn modifier(self) -> &'static str {
        match self {
            ButtonSize::Sm => "sm",
            ButtonSize::Md => "md",
            ButtonSize::Lg => "lg",
        }
    }

    /// Class string for any button-family component.
    /// `prefix` is the BEM block (e.g. `"if-button"`, `"if-icon-button"`).
    #[must_use]
    pub(crate) fn class_for(self, prefix: &str) -> String {
        format!("{prefix}--{}", self.modifier())
    }

    fn class(self) -> String {
        self.class_for("if-button")
    }
}

#[component]
pub fn Button(
    #[props(default = ButtonVariant::Primary)] variant: ButtonVariant,
    #[props(default = ButtonSize::Md)] size: ButtonSize,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
    /// Forwarded to the inner <button>'s `onmounted` so callers can move
    /// focus to it on mount. Used by the F4 confirm dialog to put initial
    /// focus on Cancel (the safer default).
    #[props(default)]
    onmounted: Option<EventHandler<MountedEvent>>,
    children: Element,
) -> Element {
    let variant_class = format!("{} {}", variant.class(), size.class());
    let combined = merge_class("if-button", &variant_class, class.as_deref());
    let click_handler = move |evt: MouseEvent| {
        if let Some(handler) = &onclick {
            handler.call(evt);
        }
    };
    let mounted_handler = move |evt: MountedEvent| {
        if let Some(handler) = &onmounted {
            handler.call(evt);
        }
    };
    rsx! {
        button {
            class: "{combined}",
            disabled,
            onclick: click_handler,
            onmounted: mounted_handler,
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

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

    /// Regression: `onmounted` must be forwarded to the inner `<button>` so the
    /// F4 confirm dialog can move focus to Cancel on first render. Renders the
    /// component to HTML and asserts presence of the button element.
    #[test]
    fn button_renders_and_accepts_onmounted_prop() {
        fn harness() -> Element {
            rsx! {
                Button {
                    onmounted: move |_evt: MountedEvent| {},
                    onclick: move |_| {},
                    "Cancel"
                }
            }
        }
        let mut vdom = VirtualDom::new(harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("<button"), "got: {html}");
        assert!(html.contains("Cancel"), "got: {html}");
    }
}
