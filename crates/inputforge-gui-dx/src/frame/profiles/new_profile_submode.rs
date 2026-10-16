#![expect(
    unused_qualifications,
    reason = "Dioxus RSX emits redundant qualifications for native event attributes"
)]
//! Rendering for the app-owned new profile form.
use crate::{
    components::{Button, ButtonSize, ButtonVariant, InputSize, TextInput},
    profiles::{Context, Intent, Source},
};
use dioxus::prelude::*;

fn label(source: &Source) -> &'static str {
    match source {
        Source::Blank => "Blank",
        Source::Active => "Copy active",
        Source::Library(_) => "Copy from library",
        Source::File => "Open existing file",
    }
}
fn description(source: &Source) -> &'static str {
    match source {
        Source::Blank => "Empty profile with one default mode.",
        Source::Active => "Duplicate the currently active profile.",
        Source::Library(_) => "Pick another library profile to duplicate.",
        Source::File => "Pick a profile file from disk.",
    }
}
#[component]
pub(crate) fn NewProfileSubMode() -> Element {
    let c = use_context::<Context>().capture();
    let view = c.view.read().clone();
    let name = use_memo(move || c.view.read().name.clone());
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
                "New profile"
            }
            div {
                class: "profiles-panel__submode-field",
                label {
                    class: "profiles-panel__submode-field-label",
                    "Name"
                }
                TextInput {
                    value: name,
                    size: InputSize::Sm,
                    placeholder: "Profile name".to_owned(),
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
            }
            fieldset {
                class: "profiles-panel__source-group",
                legend {
                    class: "profiles-panel__submode-field-label",
                    "Source"
                }
                for option in [Source::Blank, Source::Active, Source::Library(None), Source::File] {
                    {
                        let selected = std::mem::discriminant(&view.source)
                            == std::mem::discriminant(&option);
                        let source = option.clone();
                        let library = view.library.clone();
                        let source_change = move |_| {
                            let _ = c.send(Intent::Source(source.clone()));
                        };
                        let library_change = move |e: FormEvent| {
                            let _ = c.send(Intent::LibrarySource {
                                key: e.value(),
                                choices: library.clone(),
                            });
                        };
                        rsx! {
                            label {
                                class: if selected { "profiles-panel__source-option profiles-panel__source-option--selected" } else { "profiles-panel__source-option" },
                                input {
                                    r#type: "radio",
                                    name: "new-profile-source",
                                    checked: selected,
                                    onchange: source_change,
                                }
                                div {
                                    class: "profiles-panel__source-option-text",
                                    strong {
                                        "{label(&option)}"
                                    }
                                    span {
                                        class: "profiles-panel__source-option-desc",
                                        "{description(&option)}"
                                    }
                                    if matches!(option, Source::Library(_)) && selected {
                                        div {
                                            class: "profiles-panel__source-detail",
                                            select {
                                                class: "if-text-input if-text-input--sm",
                                                value: match &view.source {
                                                    Source::Library(Some(t)) => t.key.clone(),
                                                    _ => String::new(),
                                                },
                                                onchange: library_change,
                                                option {
                                                    value: "",
                                                    "Pick a profile..."
                                                }
                                                for target in &view.library {
                                                    option {
                                                        value: "{target.key}",
                                                        "{target.name}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if option == Source::File && selected {
                                        div {
                                            class: "profiles-panel__source-detail",
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Sm,
                                                onclick: move |_| {
                                                    let _ = c.send(Intent::Pick);
                                                },
                                                "Choose file..."
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div {
                class: "profiles-panel__submode-actions",
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    onclick: move |_| {
                        let _ = c.send(Intent::Create);
                    },
                    disabled: !view.can_create,
                    "Create"
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
