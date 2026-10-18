use dioxus::prelude::*;

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: EMPTY_STATE_CSS }"
)]
const EMPTY_STATE_CSS: Asset = asset!("/assets/frame/empty_state.css");

#[component]
pub(crate) fn EmptyState() -> Element {
    tracing::trace!(target: "frame::render", region = "empty_state");
    let profiles = crate::frame::profiles::use_bindings();
    let panel = use_context::<crate::frame::view_state::ViewState>().panel_slot;
    rsx! {
        Stylesheet { href: EMPTY_STATE_CSS }
        crate::components::EmptyState {
            class: "if-empty-state",
            title: "No profile loaded",
            eyebrow: rsx! { crate::components::ReadoutCaption { "Status, Standby" } },
            crate::components::Separator { decorative: true, strong: true, class: "if-empty-state__rule" }
            p { class: "if-empty-state__hint", "Create a profile or open one from disk to start configuring inputs." }
            crate::components::Cluster { gap: "--space-2", class: "if-empty-state__actions",
                crate::components::Button {
                    onclick: move |_| profiles.start.call((panel, crate::profiles::Intent::New)),
                    "New profile"
                }
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Secondary,
                    onclick: move |_| profiles.start.call((panel, crate::profiles::Intent::Pick)),
                    "Open file"
                }
            }
        }
    }
}
