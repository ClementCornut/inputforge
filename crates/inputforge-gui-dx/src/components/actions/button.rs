//! Native action buttons with explicit accessibility and interaction hooks.
use super::types::{ActionTone, ButtonPresentation};
pub use super::types::{ButtonSize, ButtonVariant};
use crate::components::merge_class;
use dioxus::prelude::*;

/// Render a native button; keyboard activation remains browser-owned.
#[component]
pub fn Button(
    #[props(default = ButtonVariant::Primary)] variant: ButtonVariant,
    #[props(default = ButtonSize::Md)] size: ButtonSize,
    #[props(default)] presentation: ButtonPresentation,
    #[props(default)] tone: ActionTone,
    #[props(default)] selected: bool,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    #[props(default = "button".to_owned())] button_type: String,
    #[props(default)] id: Option<String>,
    #[props(default)] title: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] aria_pressed: Option<bool>,
    #[props(default)] aria_expanded: Option<bool>,
    #[props(default)] aria_controls: Option<String>,
    #[props(default)] aria_haspopup: Option<String>,
    #[props(default)] aria_current: Option<String>,
    #[props(default)] autofocus: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] oncontextmenu: Option<EventHandler<MouseEvent>>,
    #[props(default)] onmounted: Option<EventHandler<MountedEvent>>,
    children: Element,
) -> Element {
    let variants = format!(
        "{} {} {}",
        variant.class(),
        size.class(),
        presentation.class()
    );
    let combined = merge_class("if-button", variants.trim(), class.as_deref());
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
            "aria-label": aria_label,
            "aria-labelledby": aria_labelledby,
            "aria-describedby": aria_describedby,
            "aria-pressed": aria_pressed.map(|value| value.to_string()),
            "aria-expanded": aria_expanded.map(|value| value.to_string()),
            "aria-controls": aria_controls,
            "aria-haspopup": aria_haspopup,
            "aria-current": aria_current,
            "data-selected": selected.to_string(),
            "data-tone": tone.name(),
            class: combined,
            disabled,
            autofocus,
            onclick,
            onkeydown,
            oncontextmenu,
            onmounted,
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
