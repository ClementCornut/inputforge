use dioxus::prelude::*;

use crate::components::merge_class;
use crate::components::{FeedbackTone, InlineFeedback, Label};

/// Form-row wrapper: label + input area + helper / error text.
///
/// To couple the label to the wrapped input for screen readers and click-to-focus,
/// pass the same string to both `for_id` (here) and the `id` prop on the wrapped
/// form input. Field forwards `for_id` to the inner `Label`'s `for=` attribute,
/// and the wrapped input must echo it via its own `id=` attribute.
///
/// Helper and error IDs are explicit: pass `helper_id` / `error_id` here and the
/// matching `aria_describedby` / `aria_errormessage` on the actual native control.
/// Pass only the mounted message's ID. An error replaces helper text; set the
/// control's `invalid` only for input validation failures. Field does not mutate children.
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
    #[props(default)] helper_id: Option<String>,
    #[props(default)] error_id: Option<String>,
    #[props(default)] horizontal: bool,
    #[props(default)] error: Option<String>,
    #[props(default)] required: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class(
        "if-field",
        if horizontal {
            "if-field--horizontal"
        } else {
            ""
        },
        class.as_deref(),
    );
    rsx! {
        div { class: "{combined}",
            Label { for_id: for_id.clone(),
                "{label}"
                if required { span { class: "if-field__required", " *" } }
            }
            div { class: "if-field__control", {children} }
            if let Some(err) = error.as_deref() {
                InlineFeedback { class: "if-field__error", id: error_id, tone: FeedbackTone::Error, alert: true, "{err}" }
            } else if let Some(h) = helper.as_deref() {
                InlineFeedback { class: "if-field__helper", id: helper_id, "{h}" }
            }
        }
    }
}

#[cfg(test)]
#[path = "field_tests.rs"]
mod tests;
