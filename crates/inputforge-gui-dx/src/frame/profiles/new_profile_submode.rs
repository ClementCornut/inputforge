//! Panel sub-mode: create a new profile.
//!
//! Replaces the library region when `ProfilesPanelMode::NewProfile` is
//! active. Carries the spec contract: back / cancel affordance, name
//! field, source choices (Blank / Copy active / Copy from library /
//! Open existing file), Create action disabled until valid.

#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion triggers false-positive unused_qualifications warnings on \
              event handler field names like onclick: and onchange:"
)]

use std::path::PathBuf;

use dioxus::prelude::*;

use crate::components::{Button, ButtonSize, ButtonVariant, InputSize, TextInput};
use crate::context::AppContext;
use crate::frame::profiles::new_profile::{NewProfileSource, create_new_profile_command};
use crate::frame::view_state::{ProfilesPanelMode, ViewState};
use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceChoice {
    Blank,
    CopyActive,
    CopyFromLibrary,
    OpenExistingFile,
}

impl SourceChoice {
    fn label(self) -> &'static str {
        match self {
            Self::Blank => "Blank",
            Self::CopyActive => "Copy active",
            Self::CopyFromLibrary => "Copy from library",
            Self::OpenExistingFile => "Open existing file",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Blank => "Empty profile with one default mode.",
            Self::CopyActive => "Duplicate the currently active profile.",
            Self::CopyFromLibrary => "Pick another library profile to duplicate.",
            Self::OpenExistingFile => "Pick a profile file from disk.",
        }
    }
}

const SOURCE_OPTIONS: [SourceChoice; 4] = [
    SourceChoice::Blank,
    SourceChoice::CopyActive,
    SourceChoice::CopyFromLibrary,
    SourceChoice::OpenExistingFile,
];

#[component]
pub(crate) fn NewProfileSubMode() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let mut name = use_signal(|| "New Profile".to_owned());
    let name_read: ReadSignal<String> = name.into();
    let mut source = use_signal(|| SourceChoice::Blank);
    let mut copy_source_path = use_signal(|| None::<PathBuf>);

    let library_rows = ctx
        .state
        .read()
        .profile_library_rows
        .iter()
        .map(|row| (row.path.clone(), row.name.clone()))
        .collect::<Vec<_>>();
    let active_path = ctx.state.read().profile_path.clone();

    let mut view_for_back = view;
    let go_to_library = move || {
        view_for_back.profiles_panel.write().mode = ProfilesPanelMode::Library;
    };
    let mut go_back_click = go_to_library;
    let mut cancel_click_handler = go_to_library;

    let trimmed_name = name.read().trim().to_owned();
    let create_enabled = match *source.read() {
        SourceChoice::Blank => !trimmed_name.is_empty(),
        SourceChoice::CopyActive => !trimmed_name.is_empty() && active_path.is_some(),
        SourceChoice::CopyFromLibrary => {
            !trimmed_name.is_empty() && copy_source_path.read().is_some()
        }
        SourceChoice::OpenExistingFile => false,
    };

    let commands = ctx.commands.clone();
    let mut view_for_create = view;
    let active_path_for_create = active_path.clone();
    let create_click = move |_| {
        let name_value = name.read().trim().to_owned();
        if name_value.is_empty() {
            return;
        }
        let result = match *source.read() {
            SourceChoice::Blank => {
                create_new_profile_command(NewProfileSource::Blank, &name_value, None)
            }
            SourceChoice::CopyActive => create_new_profile_command(
                NewProfileSource::CopyActive,
                &name_value,
                active_path_for_create.clone(),
            ),
            SourceChoice::CopyFromLibrary => copy_source_path.read().clone().map_or_else(
                || Err("pick a source profile".to_owned()),
                |path| {
                    create_new_profile_command(
                        NewProfileSource::CopyProfile(path),
                        &name_value,
                        None,
                    )
                },
            ),
            SourceChoice::OpenExistingFile => Err("use the picker to choose a path".to_owned()),
        };
        if let Ok(command) = result {
            let _ = commands.send(command);
            view_for_create.profiles_panel.write().mode = ProfilesPanelMode::Library;
        }
    };

    rsx! {
        section { class: "profiles-panel__submode",
            button {
                class: "profiles-panel__submode-back",
                r#type: "button",
                onclick: move |_| go_back_click(),
                "‹ Back to library"
            }
            div { class: "profiles-panel__submode-title", "New profile" }
            div { class: "profiles-panel__submode-field",
                label { class: "profiles-panel__submode-field-label", "Name" }
                TextInput {
                    value: name_read,
                    size: InputSize::Sm,
                    placeholder: "Profile name".to_owned(),
                    oninput: move |evt: FormEvent| name.set(evt.value()),
                }
            }
            fieldset { class: "profiles-panel__source-group",
                legend { class: "profiles-panel__submode-field-label", "Source" }
                for option in SOURCE_OPTIONS {
                    {
                        let library_for_iter = library_rows.clone();
                        let mut view_for_pick = view;
                        let commands_for_pick = ctx.commands.clone();
                        let pick_existing = move |_| {
                            let commands = commands_for_pick.clone();
                            spawn(async move {
                                if let Some(handle) = rfd::AsyncFileDialog::new()
                                    .add_filter("Profile (TOML)", &["toml"])
                                    .set_title("Open profile")
                                    .pick_file()
                                    .await
                                {
                                    let path = handle.path().to_path_buf();
                                    // C3: in-library picks load directly.
                                    let library_dir = AppSettings::profiles_dir();
                                    if path.starts_with(&library_dir) {
                                        let _ = commands.send(EngineCommand::LoadProfile(path));
                                        view_for_pick.profiles_panel.write().mode =
                                            ProfilesPanelMode::Library;
                                    } else {
                                        let suggested = path
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("Imported")
                                            .to_owned();
                                        view_for_pick.profiles_panel.write().mode =
                                            ProfilesPanelMode::OpenChoice {
                                                path,
                                                suggested_name: suggested,
                                            };
                                    }
                                }
                            });
                        };
                        rsx! {
                            label {
                                class: if *source.read() == option {
                                    "profiles-panel__source-option profiles-panel__source-option--selected"
                                } else {
                                    "profiles-panel__source-option"
                                },
                                input {
                                    r#type: "radio",
                                    name: "new-profile-source",
                                    checked: *source.read() == option,
                                    onchange: move |_| source.set(option),
                                }
                                div { class: "profiles-panel__source-option-text",
                                    strong { "{option.label()}" }
                                    span { class: "profiles-panel__source-option-desc",
                                        "{option.description()}"
                                    }
                                    if option == SourceChoice::CopyFromLibrary && *source.read() == option {
                                        div { class: "profiles-panel__source-detail",
                                            select {
                                                class: "if-text-input if-text-input--sm",
                                                onchange: move |evt: FormEvent| {
                                                    let value = evt.value();
                                                    if value.is_empty() {
                                                        copy_source_path.set(None);
                                                    } else if let Some((path, _label)) = library_for_iter
                                                        .iter()
                                                        .find(|(p, _)| p.display().to_string() == value)
                                                    {
                                                        copy_source_path.set(Some(path.clone()));
                                                    }
                                                },
                                                option { value: "", "Pick a profile..." }
                                                for (path, label) in &library_for_iter {
                                                    option { value: "{path.display()}", "{label}" }
                                                }
                                            }
                                        }
                                    }
                                    if option == SourceChoice::OpenExistingFile && *source.read() == option {
                                        div { class: "profiles-panel__source-detail",
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Sm,
                                                onclick: pick_existing,
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
            div { class: "profiles-panel__submode-actions",
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    onclick: create_click,
                    disabled: !create_enabled,
                    "Create"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    onclick: move |_| cancel_click_handler(),
                    "Cancel"
                }
            }
        }
    }
}
