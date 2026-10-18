use dioxus::prelude::*;

use crate::components::merge_class;

#[component]
pub fn Label(
    for_id: Option<String>,
    #[props(default)] id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-label", "", class.as_deref());
    // HTML5 forbids for="", so render the attribute only when Some.
    rsx! {
        if let Some(ref for_val) = for_id {
            label {
                class: "{combined}",
                id: id.clone(),
                r#for: "{for_val}",
                {children}
            }
        } else {
            label {
                class: "{combined}",
                id: id.clone(),
                {children}
            }
        }
    }
}
