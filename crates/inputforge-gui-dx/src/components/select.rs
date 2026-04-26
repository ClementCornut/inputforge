use dioxus::prelude::*;

use super::merge_class;
use crate::components::text_input::InputSize;

#[component]
pub fn Select(
    value: ReadSignal<String>,
    onchange: Option<EventHandler<FormEvent>>,
    options: Vec<(String, String)>, // (value, label)
    #[props(default)] disabled: bool,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-select--sm",
        InputSize::Md => "if-select--md",
        InputSize::Lg => "if-select--lg",
    };
    let combined = merge_class("if-select", size_class, class.as_deref());
    let change_handler = move |evt: FormEvent| {
        if let Some(h) = &onchange {
            h.call(evt);
        }
    };
    rsx! {
        select {
            class: "{combined}",
            value: "{value}",
            disabled,
            onchange: change_handler,
            for (val, label) in options.iter() {
                option { value: "{val}", "{label}" }
            }
        }
    }
}
