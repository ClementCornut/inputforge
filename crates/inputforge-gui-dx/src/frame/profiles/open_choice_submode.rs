//! Rendering for the app-owned picked-file choice.
use crate::{
    components::{
        Button, ButtonSize, ButtonVariant, Cluster, FeedbackTone, Field, InlineFeedback, InputSize,
        TextInput,
    },
    profiles::{Context, Intent},
};
use dioxus::prelude::*;
use std::path::PathBuf;
#[component]
pub(crate) fn OpenChoiceSubMode(path: PathBuf) -> Element {
    let c = use_context::<Context>().capture();
    let view = c.view.read().clone();
    let name = use_memo(move || c.view.read().name.clone());
    let path = path.display().to_string();
    let error = view.name_error;
    let invalid = error.is_some();
    let go_back = move |_| {
        let _ = c.send(Intent::Back);
    };
    rsx! {
        section {
            class: "profiles-panel__submode",
            Button {
                variant: ButtonVariant::Ghost, size: ButtonSize::Sm,
                class: "profiles-panel__submode-back",
                onclick: go_back,
                "‹ Back to library"
            }
            div {
                class: "profiles-panel__submode-title",
                "Open profile"
            }
            span {
                class: "profiles-panel__submode-path",
                title: "{path}",
                "{path}"
            }
            Field {
                label: "Library name", for_id: "open-profile-name",
                helper: "Used only when adding to library. Load once keeps the file in place.",
                helper_id: "open-profile-name-helper", error, error_id: "open-profile-name-error",
                TextInput {
                    id: "open-profile-name", value: name, size: InputSize::Sm, placeholder: "Library name",
                    invalid, aria_describedby: if invalid { "open-profile-name-error" } else { "open-profile-name-helper" },
                    aria_errormessage: invalid.then(|| "open-profile-name-error".to_owned()),
                    oninput: move |e: FormEvent| { let _ = c.send(Intent::Name(e.value())); },
                }
            }
            if let Some(error) = view.error { InlineFeedback { tone: FeedbackTone::Error, alert: true, "{error}" } }
            Cluster { gap: "--space-2",
                class: "profiles-panel__submode-actions",
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    onclick: move |_| {
                        let _ = c.send(Intent::LoadOnce);
                    },
                    "Load once"
                }
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    onclick: move |_| {
                        let _ = c.send(Intent::Add);
                    },
                    disabled: !view.can_add,
                    "Add to library"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    onclick: move |_| {
                        let _ = c.send(Intent::Back);
                    },
                    "Cancel"
                }
            }
        }
    }
}
