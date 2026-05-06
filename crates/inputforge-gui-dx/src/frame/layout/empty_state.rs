use dioxus::prelude::*;

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: EMPTY_STATE_CSS }"
)]
const EMPTY_STATE_CSS: Asset = asset!("/assets/frame/empty_state.css");

#[component]
pub(crate) fn EmptyState() -> Element {
    tracing::trace!(target: "frame::render", region = "empty_state");
    rsx! {
        Stylesheet { href: EMPTY_STATE_CSS }
        div { class: "if-empty-state",
            div { class: "if-empty-state__eyebrow", "Status, Standby" }
            div { class: "if-empty-state__heading", "No profile loaded" }
            div { class: "if-empty-state__rule", "aria-hidden": "true" }
            div { class: "if-empty-state__hint", "F13 owns this surface." }
            div { class: "if-empty-state__actions",
                button { class: "button button--primary", "data-action": "new-profile", "New profile" }
                button { class: "button", "data-action": "open-profile", "Open file" }
            }
        }
    }
}
