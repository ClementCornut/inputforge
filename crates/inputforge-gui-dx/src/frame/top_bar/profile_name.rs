use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::view_state::{PanelSlot, ViewState};

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub(crate) fn ProfileName() -> Element {
    tracing::trace!(target: "frame::render", region = "profile_name");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let name = use_memo(move || ctx.meta.read().profile_name.clone());

    let n = name.read().clone();

    rsx! {
        match n {
            Some(s) => rsx! {
                button {
                    r#type: "button",
                    class: "if-profile-name if-profile-name--loaded",
                    "aria-label": format!("Profile: {s}"),
                    onclick: move |_| {
                        let mut slot = view.panel_slot;
                        slot.set(PanelSlot::Profiles);
                    },
                    span { class: "if-profile-name__role", "profile" }
                    span { class: "if-profile-name__value", "{s}" }
                }
            },
            // Empty state mirrors the loaded-state route: clicking
            // opens the Profiles side panel, the surface where a
            // profile is actually chosen. Sticking with a `<span>`
            // dead-ended the user when no profile was loaded — the
            // tools-cluster Profiles button was the only path
            // forward and not visually paired with this readout.
            None => rsx! {
                button {
                    r#type: "button",
                    class: "if-profile-name if-profile-name--empty",
                    "aria-label": "Choose profile",
                    onclick: move |_| {
                        let mut slot = view.panel_slot;
                        slot.set(PanelSlot::Profiles);
                    },
                    span { class: "if-profile-name__role", "profile" }
                    span { class: "if-profile-name__value", "no profile loaded" }
                }
            },
        }
    }
}
