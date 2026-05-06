use dioxus::prelude::*;

#[component]
pub(crate) fn NoProfileBar() -> Element {
    rsx! {
        div { class: "profiles-no-profile",
            strong { "No profile loaded" }
            div { class: "profiles-no-profile__actions",
                button { class: "button button--primary", "data-action": "new-profile", "New profile" }
                button { class: "button", "data-action": "open-profile", "Open file" }
            }
        }
    }
}
