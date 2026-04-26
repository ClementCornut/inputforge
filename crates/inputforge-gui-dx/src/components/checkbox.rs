use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Checkbox(
    checked: ReadSignal<bool>,
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] indeterminate: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let variant_class = if indeterminate {
        "if-checkbox--indeterminate"
    } else {
        ""
    };
    let combined = merge_class("if-checkbox", variant_class, class.as_deref());
    let change_handler = move |evt: FormEvent| {
        if let Some(h) = &onchange {
            h.call(evt);
        }
    };
    rsx! {
        label { class: "{combined}",
            input {
                r#type: "checkbox",
                class: "if-checkbox__input",
                checked: "{checked}",
                disabled,
                onchange: change_handler,
            }
            span { class: "if-checkbox__box" }
        }
    }
}
