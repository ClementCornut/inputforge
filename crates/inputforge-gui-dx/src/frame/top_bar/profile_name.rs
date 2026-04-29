use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::view_state::{PanelSlot, ViewState};

#[component]
pub(crate) fn ProfileName() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let name = use_memo(move || ctx.meta.read().profile_name.clone());

    let n = name.read().clone();
    let onclick = move |_| {
        let mut slot = view.panel_slot;
        slot.set(PanelSlot::Profiles);
    };

    rsx! {
        match n {
            Some(s) => rsx! {
                button {
                    r#type: "button",
                    class: "if-profile-name if-profile-name--loaded",
                    onclick,
                    "{s}"
                }
            },
            None => rsx! {
                span {
                    class: "if-profile-name if-profile-name--empty",
                    "no profile loaded"
                }
            },
        }
    }
}
