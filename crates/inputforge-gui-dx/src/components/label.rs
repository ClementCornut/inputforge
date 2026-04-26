use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Label(
    for_id: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-label", "", class.as_deref());
    rsx! {
        label {
            class: "{combined}",
            r#for: for_id.as_deref().unwrap_or(""),
            {children}
        }
    }
}
