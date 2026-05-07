use dioxus::prelude::*;

use crate::components::{Button, ButtonSize, ButtonVariant};

#[component]
pub(crate) fn NoProfileBar(
    on_new_profile: EventHandler<MouseEvent>,
    on_open_file: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "profiles-no-profile",
            strong { "No profile loaded" }
            p { class: "profiles-no-profile__hint",
                "Create a new profile or open one from disk to start configuring inputs."
            }
            div { class: "profiles-no-profile__actions",
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    onclick: move |evt| on_new_profile.call(evt),
                    "+ New profile"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    onclick: move |evt| on_open_file.call(evt),
                    "Open file..."
                }
            }
        }
    }
}
