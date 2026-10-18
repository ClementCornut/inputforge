//! Profile readout opens the app-owned Profiles panel.
use crate::{
    components::{Button, ButtonSize, ButtonVariant, ReadoutCaption},
    context::AppContext,
};
use dioxus::prelude::*;
#[component]
pub(crate) fn ProfileName() -> Element {
    let ctx = use_context::<AppContext>();
    let navigation = use_context::<crate::mapping_edits::Bindings>().navigation;
    let name = use_memo(move || ctx.meta.read().profile_name.clone());
    let name = name.read().clone();
    let label = name
        .as_ref()
        .map_or_else(|| "Choose profile".to_owned(), |s| format!("Profile: {s}"));
    let value = name.unwrap_or_else(|| "no profile loaded".to_owned());
    rsx! {
        Button {
            variant: ButtonVariant::Ghost, size: ButtonSize::Sm,
            class: "if-profile-name", aria_label: label,
            onclick: move |_| navigation.dispatch.call(crate::navigation::Intent::OpenPanel(crate::PanelSlot::Profiles)),
            ReadoutCaption { "profile" }
            span { "{value}" }
        }
    }
}
