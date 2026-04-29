use dioxus::prelude::*;

#[expect(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: EMPTY_STATE_CSS }"
)]
const EMPTY_STATE_CSS: Asset = asset!("/assets/frame/empty_state.css");

#[component]
pub(crate) fn EmptyState() -> Element {
    rsx! {
        Stylesheet { href: EMPTY_STATE_CSS }
        div { class: "if-empty-state",
            div { class: "if-empty-state__heading", "No profile loaded" }
            div { class: "if-empty-state__hint", "F13 owns this surface." }
        }
    }
}
