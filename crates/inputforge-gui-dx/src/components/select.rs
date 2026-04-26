use dioxus::prelude::*;

use super::merge_class;
use crate::components::text_input::InputSize;

#[component]
pub fn Select(
    value: ReadSignal<String>,
    onchange: Option<EventHandler<FormEvent>>,
    options: Vec<(String, String)>, // (value, label)
    #[props(default)] disabled: bool,
    /// HTML `id` for label↔input coupling when wrapped in `Field`.
    #[props(default)]
    id: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-select--sm",
        InputSize::Md => "if-select--md",
        InputSize::Lg => "if-select--lg",
    };
    let combined = merge_class("if-select", size_class, class.as_deref());
    let id_attr = id.clone().unwrap_or_default();
    let change_handler = move |evt: FormEvent| {
        if let Some(h) = &onchange {
            h.call(evt);
        }
    };
    rsx! {
        select {
            class: "{combined}",
            id: "{id_attr}",
            value: "{value}",
            disabled,
            onchange: change_handler,
            for (val, label) in options.iter() {
                option { value: "{val}", "{label}" }
            }
        }
    }
}
