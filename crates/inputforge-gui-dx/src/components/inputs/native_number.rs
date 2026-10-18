//! Raw native numeric presentation for app-owned validation on native change.
use crate::components::{InputSize, merge_class};
use dioxus::prelude::*;

/// Native numeric input forwarding the exact `onchange` event without parsing or clamping.
///
/// Format integer values directly with `to_string`; bounds retain their native string form.
/// For parsed Enter/blur commits, use `IntegerInput` or `NumberInput` instead.
#[component]
pub fn NativeNumberInput(
    value: String,
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] min: Option<String>,
    #[props(default)] max: Option<String>,
    #[props(default)] step: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] id: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] aria_errormessage: Option<String>,
    #[props(default)] invalid: bool,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-native-number--sm",
        InputSize::Md => "if-native-number--md",
        InputSize::Lg => "if-native-number--lg",
    };
    let classes = merge_class("if-native-number", size_class, class.as_deref());
    let onchange = move |event| {
        if let Some(handler) = onchange {
            handler.call(event);
        }
    };
    rsx! {
        input {
            r#type: "number",
            class: classes,
            value,
            min,
            max,
            step,
            disabled,
            id,
            "aria-label": aria_label,
            "aria-labelledby": aria_labelledby,
            "aria-describedby": aria_describedby,
            "aria-invalid": invalid.then_some("true"),
            "aria-errormessage": aria_errormessage.filter(|_| invalid),
            onchange,
        }
    }
}
