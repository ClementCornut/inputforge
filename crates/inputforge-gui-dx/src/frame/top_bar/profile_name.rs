use dioxus::prelude::*;

#[component]
pub(crate) fn ProfileName() -> Element {
    rsx! { span { class: "if-profile-name if-profile-name--placeholder", "Profile" } }
}
