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
) -> Element {
    let variant_class = if invalid {
        format!("{} if-text-input--invalid", size.class())
    } else {
        size.class().to_owned()
    };
    let classes = merge_class("if-text-input", &variant_class, class.as_deref());
    let id_attr = id.clone().unwrap_or_default();
    let input_handler = move |evt: FormEvent| {
        if let Some(h) = &oninput {
            h.call(evt);
        }
    };
    rsx! {
        input {
            r#type: "text",
            class: "{classes}",
            id: "{id_attr}",
            value: "{value}",
            placeholder: placeholder.as_deref().unwrap_or(""),
            disabled,
            oninput: input_handler,
        }
    }
}
