use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSize {
    Sm,
    Md,
    Lg,
}

impl InputSize {
    #[must_use]
    pub(crate) fn class(self) -> &'static str {
        match self {
            InputSize::Sm => "if-text-input--sm",
            InputSize::Md => "if-text-input--md",
            InputSize::Lg => "if-text-input--lg",
        }
    }
}

#[component]
pub fn TextInput(
    value: ReadSignal<String>,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] invalid: bool,
    /// HTML `id` for label↔input coupling when wrapped in `Field`.
    #[props(default)]
    id: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
    /// Forwarded to the inner <input> as `aria-describedby`. Used by inline
    /// editors to wire validation error spans (`role="alert"`) so AT users
    /// hear the message when typing produces an invalid name.
    #[props(default)]
    aria_describedby: Option<String>,
    /// Forwarded to the inner <input>'s `onmounted` so callers can move
    /// focus to it on appearance (e.g., inline-rename / inline-add open).
    #[props(default)]
    onmounted: Option<EventHandler<MountedEvent>>,
) -> Element {
    let variant_class = if invalid {
        format!("{} if-text-input--invalid", size.class())
    } else {
        size.class().to_owned()
    };
    let classes = merge_class("if-text-input", &variant_class, class.as_deref());
    let input_handler = move |evt: FormEvent| {
        if let Some(h) = &oninput {
            h.call(evt);
        }
    };
    let mounted_handler = move |evt: MountedEvent| {
        if let Some(handler) = &onmounted {
            handler.call(evt);
        }
    };
    let described_by = aria_describedby.clone().unwrap_or_default();
    // HTML5 forbids id="" — so render the attribute only when Some.
    rsx! {
        if let Some(ref id_val) = id {
            input {
                r#type: "text",
                class: "{classes}",
                id: "{id_val}",
                value: "{value}",
                placeholder: placeholder.as_deref().unwrap_or(""),
                disabled,
                "aria-describedby": "{described_by}",
                oninput: input_handler,
                onmounted: mounted_handler,
            }
        } else {
            input {
                r#type: "text",
                class: "{classes}",
                value: "{value}",
                placeholder: placeholder.as_deref().unwrap_or(""),
                disabled,
                "aria-describedby": "{described_by}",
                oninput: input_handler,
                onmounted: mounted_handler,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    #[test]
    fn text_input_forwards_aria_describedby() {
        fn harness() -> Element {
            let v: Signal<String> = use_signal(String::new);
            let v_ro: ReadSignal<String> = v.into();
            rsx! {
                TextInput {
                    value: v_ro,
                    aria_describedby: "err-id".to_owned(),
                    oninput: move |_| {},
                }
            }
        }
        let mut vdom = VirtualDom::new(harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("aria-describedby=\"err-id\""), "got: {html}");
    }
}
