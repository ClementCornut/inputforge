use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Switch(
    checked: ReadSignal<bool>,
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] label: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-switch", "", class.as_deref());
    let change_handler = move |evt: FormEvent| {
        if let Some(h) = &onchange {
            h.call(evt);
        }
    };
    rsx! {
        label { class: "{combined}",
            input {
                r#type: "checkbox",
                class: "if-switch__input",
                checked: "{checked}",
                disabled,
                onchange: change_handler,
            }
            span { class: "if-switch__track", span { class: "if-switch__thumb" } }
            if let Some(l) = label.as_deref() {
                span { class: "if-switch__label", "{l}" }
            }
        }
    }
}
