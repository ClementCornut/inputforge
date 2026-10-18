//! Rendering for the app-owned new profile form.
use crate::{
    components::{
        Button, ButtonSize, ButtonVariant, Cluster, FeedbackTone, Field, InlineFeedback, InputSize,
        TextInput,
    },
    profiles::{Context, Intent},
};
use dioxus::prelude::*;

#[component]
pub(crate) fn NewProfileSubMode() -> Element {
    let c = use_context::<Context>().capture();
    let view = c.view.read().clone();
    let name = use_memo(move || c.view.read().name.clone());
    let error = view.name_error;
    let invalid = error.is_some();
    rsx! {
        section { class: "profiles-panel__submode",
            Button {
                class: "profiles-panel__submode-back", variant: ButtonVariant::Ghost, size: ButtonSize::Sm,
                onclick: move |_| { let _ = c.send(Intent::Back); },
                "‹ Back to library"
            }
            div { class: "profiles-panel__submode-title", "New profile" }
            Field {
                label: "Name", for_id: "new-profile-name", error, error_id: "new-profile-name-error",
                TextInput {
                    id: "new-profile-name", value: name, size: InputSize::Sm, placeholder: "Profile name",
                    invalid, aria_describedby: invalid.then(|| "new-profile-name-error".to_owned()),
                    aria_errormessage: invalid.then(|| "new-profile-name-error".to_owned()),
                    oninput: move |e: FormEvent| { let _ = c.send(Intent::Name(e.value())); },
                }
            }
            if let Some(error) = view.error { InlineFeedback { tone: FeedbackTone::Error, alert: true, "{error}" } }
            super::source_options::SourceOptions {}
            Cluster { class: "profiles-panel__submode-actions", gap: "--space-2",
                Button { size: ButtonSize::Sm, disabled: !view.can_create,
                    onclick: move |_| { let _ = c.send(Intent::Create); }, "Create"
                }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm,
                    onclick: move |_| { let _ = c.send(Intent::Back); }, "Cancel"
                }
            }
        }
    }
}
