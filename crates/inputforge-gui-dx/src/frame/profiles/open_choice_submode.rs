#![expect(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications for native event attributes"
)]
//! Rendering for the app-owned picked-file choice.
use crate::{
    components::{Button, ButtonSize, ButtonVariant, InputSize, TextInput},
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
    let go_back = move |_| {
        let _ = c.send(Intent::Back);
    };
    rsx! {
        section {
            class: "profiles-panel__submode",
            button {
                class: "profiles-panel__submode-back",
                r#type: "button",
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
            div {
                class: "profiles-panel__submode-field",
                label {
                    class: "profiles-panel__submode-field-label",
                    "Library name"
                }
                TextInput {
                    value: name,
                    size: InputSize::Sm,
                    placeholder: "Library name".to_owned(),
                    oninput: move |e: FormEvent| {
                        let _ = c.send(Intent::Name(e.value()));
                    },
                }
                if let Some(error) = view.error.or(view.name_error) {
                    div {
                        class: "profiles-panel__submode-error",
                        role: "alert",
                        "{error}"
                    }
                }
                p {
                    class: "profiles-panel__submode-hint",
                    "Used only when adding to library. Load once keeps the file in place."
                }
            }
            div {
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
