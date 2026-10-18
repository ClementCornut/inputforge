//! Native source choices keep source-specific controls outside radio labels.
use crate::{
    components::{
        Button, ButtonSize, ButtonVariant, InputSize, RadioGroup, RadioOption, Select, SelectOption,
    },
    profiles::{Context, Intent, Source},
};
use dioxus::prelude::*;

#[component]
pub(super) fn SourceOptions() -> Element {
    let c = use_context::<Context>().capture();
    let view = c.view.read().clone();
    let selected_key = use_memo(move || match &c.view.read().source {
        Source::Library(Some(target)) => target.key.clone(),
        _ => String::new(),
    });
    rsx! {
        RadioGroup { label: "Source",
            for (value, label, description, option) in [
                ("blank", "Blank", "Empty profile with one default mode.", Source::Blank),
                ("active", "Copy active", "Duplicate the currently active profile.", Source::Active),
                ("library", "Copy from library", "Pick another library profile to duplicate.", Source::Library(None)),
                ("file", "Open existing file", "Pick a profile file from disk.", Source::File),
            ] {
                {
                    let selected = std::mem::discriminant(&view.source) == std::mem::discriminant(&option);
                    let source = option.clone();
                    let choices = view.library.clone();
                    let mut options = vec![SelectOption { value: String::new(), label: "Pick a profile...".to_owned(), disabled: false, class: None }];
                    options.extend(view.library.iter().map(|target| SelectOption { value: target.key.clone(), label: target.name.clone(), disabled: false, class: None }));
                    rsx! {
                        RadioOption {
                            name: "new-profile-source", value, label, description, checked: selected,
                            onchange: move |_| { let _ = c.send(Intent::Source(source.clone())); },
                            details: rsx! {
                                if selected && matches!(option, Source::Library(_)) {
                                    div { class: "profiles-panel__source-detail",
                                        Select { value: selected_key, size: InputSize::Sm, aria_label: "Library source profile", options,
                                            onchange: move |e: FormEvent| { let _ = c.send(Intent::LibrarySource { key: e.value(), choices: choices.clone() }); },
                                        }
                                    }
                                }
                                if selected && option == Source::File {
                                    div { class: "profiles-panel__source-detail",
                                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm,
                                            onclick: move |_| { let _ = c.send(Intent::Pick); }, "Choose file..."
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}
