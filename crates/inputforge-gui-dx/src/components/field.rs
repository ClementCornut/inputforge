use dioxus::prelude::*;

use super::merge_class;
use crate::components::Label;

/// Form-row wrapper: label + input area + helper / error text.
///
/// To couple the label to the wrapped input for screen readers and click-to-focus,
/// pass the same string to both `for_id` (here) and the `id` prop on the wrapped
/// form input. Field forwards `for_id` to the inner `Label`'s `for=` attribute,
/// and the wrapped input must echo it via its own `id=` attribute.
///
/// ```ignore
/// Field {
///     label: "Profile name".to_owned(),
///     for_id: "profile-name".to_owned(),
///     TextInput { id: "profile-name".to_owned(), value: "...".to_owned() }
/// }
/// ```
#[component]
pub fn Field(
    label: String,
    #[props(default)] for_id: Option<String>,
    #[props(default)] helper: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] required: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-field", "", class.as_deref());
    rsx! {
        div { class: "{combined}",
            Label { for_id: for_id.clone(),
                "{label}"
                if required { span { class: "if-field__required", " *" } }
            }
            div { class: "if-field__control", {children} }
            if let Some(err) = error.as_deref() {
                span { class: "if-field__error", role: "alert", "{err}" }
            } else if let Some(h) = helper.as_deref() {
                span { class: "if-field__helper", "{h}" }
            }
        }
    }
}
