//! Native radio choices keep additional interactive details outside each option label.
use crate::components::merge_class;
use dioxus::prelude::*;

/// Native fieldset and legend for a group of mutually exclusive choices.
#[component]
pub fn RadioGroup(
    label: String,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let classes = merge_class("if-radio-group", "", class.as_deref());
    rsx! { fieldset { class: classes, disabled, legend { class: "if-radio-group__legend", "{label}" } {children} } }
}

/// Native radio option with a separate detail slot for selected-source controls.
#[component]
pub fn RadioOption(
    name: String,
    value: String,
    label: String,
    checked: bool,
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] description: Option<String>,
    #[props(default = VNode::empty())] details: Element,
    #[props(default)] disabled: bool,
    #[props(default)] id: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] aria_errormessage: Option<String>,
    #[props(default)] invalid: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let classes = merge_class(
        "if-radio-option",
        if checked {
            "if-radio-option--selected"
        } else {
            ""
        },
        class.as_deref(),
    );
    let onchange = move |event| {
        if let Some(handler) = onchange {
            handler.call(event);
        }
    };
    rsx! {
        div { class: classes,
            label { class: "if-radio-option__label",
                input { r#type: "radio", name, value, checked, disabled, id,
                    "aria-label": aria_label,
                    "aria-labelledby": aria_labelledby,
                    "aria-describedby": aria_describedby,
                    "aria-invalid": invalid.then_some("true"),
                    "aria-errormessage": aria_errormessage.filter(|_| invalid),
                    onchange,
                }
                span { class: "if-radio-option__text",
                    strong { "{label}" }
                    if let Some(description) = description { span { class: "if-radio-option__description", "{description}" } }
                }
            }
            {details}
        }
    }
}
