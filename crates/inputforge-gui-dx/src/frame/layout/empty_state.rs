use dioxus::prelude::*;

#[component]
pub(crate) fn EmptyState() -> Element {
    rsx! {
        div { class: "if-empty-state",
            div { class: "if-empty-state__heading", "No profile loaded" }
            div { class: "if-empty-state__hint", "F13 owns this surface." }
        }
    }
}
