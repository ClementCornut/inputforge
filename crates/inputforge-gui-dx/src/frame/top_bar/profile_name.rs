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
            None => rsx! {
                span {
                    class: "if-profile-name if-profile-name--empty",
                    span { class: "if-profile-name__role", "profile" }
                    span { class: "if-profile-name__value", "no profile loaded" }
                }
            },
        }
    }
}
