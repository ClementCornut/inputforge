//! Text inputs preserve app-owned draft transport and forward native accessibility metadata.
use crate::components::merge_class;
use dioxus::prelude::*;

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
            Self::Sm => "if-text-input--sm",
            Self::Md => "if-text-input--md",
            Self::Lg => "if-text-input--lg",
        }
    }
}

/// Text field appearance for ordinary forms and in-place editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextInputVariant {
    #[default]
    Default,
    Inline,
    Title,
}

/// Native text field with app-owned text acknowledgement and browser-owned editing.
///
/// Pass IDREF metadata only while the corresponding label/helper/error is mounted.
#[component]
pub fn TextInput(
    value: ReadSignal<String>,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] invalid: bool,
    #[props(default)] id: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] aria_errormessage: Option<String>,
    #[props(default)] data_editor_focus: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] variant: TextInputVariant,
    #[props(default)] class: Option<String>,
    #[props(default)] onmounted: Option<EventHandler<MountedEvent>>,
    #[props(default)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] onblur: Option<EventHandler<FocusEvent>>,
    #[props(default)] onfocus: Option<EventHandler<FocusEvent>>,
) -> Element {
    let variant_class = match variant {
        TextInputVariant::Default => "",
        TextInputVariant::Inline => "if-text-input--inline",
        TextInputVariant::Title => "if-text-input--title",
    };
    let variants = merge_class(
        size.class(),
        variant_class,
        invalid.then_some("if-text-input--invalid"),
    );
    let classes = merge_class("if-text-input", &variants, class.as_deref());
    let text = (crate::components::behavior().text)(crate::controls::TextProps {
        value,
        disabled,
        oninput,
    });
    let behavior = crate::components::behavior();
    let onmounted = move |event| {
        if let Some(handler) = onmounted {
            handler.call(event);
        }
    };
    let onkeydown = move |event| behavior.text_key.call((event, disabled, onkeydown));
    let onblur = move |event| {
        if let Some(handler) = onblur {
            handler.call(event);
        }
    };
    let onfocus = move |event| {
        if let Some(handler) = onfocus {
            handler.call(event);
        }
    };
    rsx! {
        input {
            r#type: "text",
            class: classes,
            id,
            initial_value: "{value}",
            "data-if-text-owner": "{text.owner}",
            "data-editor-focus": data_editor_focus,
            placeholder,
            disabled,
            "aria-label": aria_label,
            "aria-labelledby": aria_labelledby,
            "aria-describedby": aria_describedby,
            "aria-invalid": invalid.then_some("true"),
            "aria-errormessage": aria_errormessage.filter(|_| invalid),
            oninput: text.input,
            onmounted,
            onkeydown,
            onblur,
            onfocus,
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
