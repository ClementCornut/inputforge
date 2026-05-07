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
    /// Forwarded to the inner `<input>` as `aria-describedby` ONLY when
    /// `Some`. HTML5 forbids empty IDREFs and dangling IDREFs, so callers
    /// must omit the prop entirely (or pass `None`) until the described
    /// element is actually mounted in the DOM. Inline editors should pass
    /// `error_msg.read().is_some().then_some(error_id.to_owned())` so the
    /// IDREF only appears alongside the error span it points at.
    #[props(default)]
    aria_describedby: Option<String>,
    /// Forwarded to the inner <input>'s `onmounted` so callers can move
    /// focus to it on appearance (e.g., inline-rename / inline-add open).
    #[props(default)]
    onmounted: Option<EventHandler<MountedEvent>>,
    /// Forwarded to the inner <input>'s `onkeydown` so inline editors
    /// (rename / new-profile) can implement Enter-commits and Esc-cancels.
    #[props(default)]
    onkeydown: Option<EventHandler<KeyboardEvent>>,
    /// Forwarded to the inner <input>'s `onblur` so inline editors can
    /// implement blur-commits.
    #[props(default)]
    onblur: Option<EventHandler<FocusEvent>>,
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
    let keydown_handler = move |evt: KeyboardEvent| {
        if let Some(handler) = &onkeydown {
            handler.call(evt);
        }
    };
    let blur_handler = move |evt: FocusEvent| {
        if let Some(handler) = &onblur {
            handler.call(evt);
        }
    };
    // Pass `Option<String>` directly so Dioxus omits the attribute when
    // `None` (vs `""`). Same pattern as `id` below, both attributes have
    // HTML5 IDREF/empty-string constraints that an unconditional template
    // string would violate.
    let described_by = aria_describedby.clone();
    rsx! {
        if let Some(ref id_val) = id {
            input {
                r#type: "text",
                class: "{classes}",
                id: "{id_val}",
                value: "{value}",
                placeholder: placeholder.as_deref().unwrap_or(""),
                disabled,
                "aria-describedby": described_by,
                oninput: input_handler,
                onmounted: mounted_handler,
                onkeydown: keydown_handler,
                onblur: blur_handler,
            }
        } else {
            input {
                r#type: "text",
                class: "{classes}",
                value: "{value}",
                placeholder: placeholder.as_deref().unwrap_or(""),
                disabled,
                "aria-describedby": described_by,
                oninput: input_handler,
                onmounted: mounted_handler,
                onkeydown: keydown_handler,
                onblur: blur_handler,
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

    #[test]
    fn text_input_omits_aria_describedby_when_none() {
        // HTML5 forbids dangling/empty IDREFs. When the caller doesn't
        // provide `aria_describedby`, the attribute must be absent -
        // not rendered as `aria-describedby=""`.
        fn harness() -> Element {
            let v: Signal<String> = use_signal(String::new);
            let v_ro: ReadSignal<String> = v.into();
            rsx! {
                TextInput {
                    value: v_ro,
                    oninput: move |_| {},
                }
            }
        }
        let mut vdom = VirtualDom::new(harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("aria-describedby"),
            "expected no aria-describedby attribute when prop is None, got: {html}"
        );
    }
}
