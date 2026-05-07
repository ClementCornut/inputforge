//! Panel sub-mode: choose between Load once and Add to library after
//! the OS file picker resolves with a profile path.
//!
//! Activated via `ProfilesPanelMode::OpenChoice { path, suggested_name }`.
//! Replaces the library region with a back chevron, the picked path
//! (truncated), an editable library-name field, and the two action
//! buttons mandated by the spec lines 119-122.

#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion triggers false-positive unused_qualifications warnings on \
              event handler field names like onclick:"
)]

use std::path::PathBuf;

use dioxus::prelude::*;

use crate::components::{Button, ButtonSize, ButtonVariant, InputSize, TextInput};
use crate::context::AppContext;
use crate::frame::profiles::actions::NewProfileValidationError;
use crate::frame::profiles::new_profile::{
    add_external_to_library_command, open_file_load_once_command,
};
use crate::frame::view_state::{ProfilesPanelMode, ViewState};

#[component]
pub(crate) fn OpenChoiceSubMode(path: PathBuf, suggested_name: String) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let mut name = use_signal(|| suggested_name.clone());
    let name_read: ReadSignal<String> = name.into();
    let mut error = use_signal(|| None::<NewProfileValidationError>);

    let existing_names: Vec<String> = ctx
        .state
        .read()
        .profile_library_rows
        .iter()
        .map(|row| row.name.clone())
        .collect();

    let mut view_for_back = view;
    let go_to_library = move || {
        view_for_back.profiles_panel.write().mode = ProfilesPanelMode::Library;
    };
    let mut go_back_click = go_to_library;
    let mut cancel_click = go_to_library;

    let path_for_load = path.clone();
    let commands_load = ctx.commands.clone();
    let mut view_for_load = view;
    let load_once_click = move |_| match open_file_load_once_command(path_for_load.clone()) {
        Ok(cmd) => {
            error.set(None);
            let _ = commands_load.send(cmd);
            view_for_load.profiles_panel.write().mode = ProfilesPanelMode::Library;
        }
        Err(err) => error.set(Some(err)),
    };

    let path_for_add = path.clone();
    let commands_add = ctx.commands.clone();
    let mut view_for_add = view;
    let existing_for_add = existing_names.clone();
    let add_to_library_click = move |_| {
        let name_value = name.read().clone();
        match add_external_to_library_command(path_for_add.clone(), &name_value, &existing_for_add)
        {
            Ok(cmd) => {
                error.set(None);
                let _ = commands_add.send(cmd);
                view_for_add.profiles_panel.write().mode = ProfilesPanelMode::Library;
            }
            Err(err) => error.set(Some(err)),
        }
    };

    let path_display = path.display().to_string();
    let trimmed_name = name.read().trim().to_owned();
    let add_enabled = !trimmed_name.is_empty();

    rsx! {
        section { class: "profiles-panel__submode",
            button {
                class: "profiles-panel__submode-back",
                r#type: "button",
                onclick: move |_| go_back_click(),
                "‹ Back to library"
            }
            div { class: "profiles-panel__submode-title", "Open profile" }
            span { class: "profiles-panel__submode-path", title: "{path_display}", "{path_display}" }
            div { class: "profiles-panel__submode-field",
                label { class: "profiles-panel__submode-field-label", "Library name" }
                TextInput {
                    value: name_read,
                    size: InputSize::Sm,
                    placeholder: "Library name".to_owned(),
                    oninput: move |evt: FormEvent| {
                        name.set(evt.value());
                        if error.read().is_some() {
                            error.set(None);
                        }
                    },
                }
                if let Some(err) = error.read().as_ref() {
                    div { class: "profiles-panel__submode-error", "{err.user_message()}" }
                }
                p { class: "profiles-panel__submode-hint",
                    "Used only when adding to library. Load once keeps the file in place."
                }
            }
            div { class: "profiles-panel__submode-actions",
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    onclick: load_once_click,
                    "Load once"
                }
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    onclick: add_to_library_click,
                    disabled: !add_enabled,
                    "Add to library"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    onclick: move |_| cancel_click(),
                    "Cancel"
                }
            }
        }
    }
}
