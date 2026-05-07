use std::path::PathBuf;

use dioxus::prelude::*;

use crate::components::{Button, ButtonSize, ButtonVariant, InputSize, TextInput};
use crate::context::AppContext;
use crate::context::MetaSnapshot;
use crate::frame::profiles::library::ProfileLibrary;
use crate::frame::profiles::new_profile::{
    NewProfileSource, add_external_to_library_command, create_new_profile_command,
    open_file_load_once_command,
};
use crate::frame::profiles::no_profile::NoProfileBar;
use crate::frame::profiles::snapshot_drawer::SnapshotDrawer;

pub(crate) mod actions;
pub(crate) mod library;
pub(crate) mod new_profile;
pub(crate) mod no_profile;
pub(crate) mod projection;
pub(crate) mod snapshot_drawer;

const PROFILES_CSS: Asset = asset!("/assets/frame/profiles.css");

#[component]
pub(crate) fn ProfilesPanel() -> Element {
    let ctx = use_context::<AppContext>();
    let mut new_profile_open = use_signal(|| false);
    let mut new_profile_name = use_signal(String::new);
    let new_profile_name_read: ReadSignal<String> = new_profile_name.into();
    let mut filter = use_signal(String::new);
    let filter_read: ReadSignal<String> = filter.into();
    let mut pending_open: Signal<Option<PathBuf>> = use_signal(|| None);
    let mut pending_open_name = use_signal(String::new);
    let pending_open_name_read: ReadSignal<String> = pending_open_name.into();

    let state = ctx.state.read();
    let snapshot_count = state.active_snapshot_rows.len();
    let has_profile = state.active_profile.is_some();
    let active_profile_name = state
        .active_profile
        .as_ref()
        .map(|profile| profile.name().to_owned())
        .unwrap_or_default();
    let meta = MetaSnapshot::from_state(&state);
    drop(state);

    let open_new_profile = move |_| {
        new_profile_name.set("New Profile".to_owned());
        new_profile_open.set(true);
    };
    let cancel_new_profile = move |_| new_profile_open.set(false);
    let commands_create = ctx.commands.clone();
    let create_profile = move |_| {
        let name = new_profile_name.read().clone();
        if let Ok(command) = create_new_profile_command(NewProfileSource::Blank, &name, None) {
            let _ = commands_create.send(command);
            new_profile_open.set(false);
        }
    };

    let open_file_click = move |_| {
        spawn(async move {
            if let Some(handle) = rfd::AsyncFileDialog::new()
                .add_filter("Profile (TOML)", &["toml"])
                .set_title("Open profile")
                .pick_file()
                .await
            {
                let path = handle.path().to_path_buf();
                let suggested = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Imported")
                    .to_owned();
                pending_open_name.set(suggested);
                pending_open.set(Some(path));
            }
        });
    };

    let commands_load_once = ctx.commands.clone();
    let load_once_click = move |_| {
        let maybe_path = pending_open.read().clone();
        if let Some(path) = maybe_path {
            let _ = commands_load_once.send(open_file_load_once_command(path));
            pending_open.set(None);
        }
    };
    let commands_add_to_lib = ctx.commands.clone();
    let add_to_library_click = move |_| {
        let maybe_path = pending_open.read().clone();
        let Some(path) = maybe_path else {
            return;
        };
        let name = pending_open_name.read().clone();
        if let Ok(cmd) = add_external_to_library_command(path, &name) {
            let _ = commands_add_to_lib.send(cmd);
            pending_open.set(None);
        }
    };
    let cancel_open = move |_| pending_open.set(None);

    rsx! {
        Stylesheet { href: PROFILES_CSS }
        section { class: "profiles-panel", "data-testid": "profile-library",
            header { class: "profiles-panel__header",
                h2 { "Profiles" }
                div { class: "profiles-panel__header-actions",
                    Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        onclick: open_new_profile,
                        "+ New profile"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        onclick: open_file_click,
                        "Open file..."
                    }
                }
            }
            if *new_profile_open.read() {
                div { class: "profiles-panel__new-profile",
                    TextInput {
                        value: new_profile_name_read,
                        size: InputSize::Sm,
                        placeholder: "Profile name".to_owned(),
                        oninput: move |evt: FormEvent| new_profile_name.set(evt.value()),
                    }
                    Button { variant: ButtonVariant::Primary, size: ButtonSize::Sm, onclick: create_profile, "Create" }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: cancel_new_profile, "Cancel" }
                }
            }
            if let Some(path) = pending_open.read().clone() {
                div { class: "profiles-panel__open-choice",
                    span { class: "profiles-panel__open-choice-label",
                        strong { "Open " }
                        span { class: "profiles-panel__open-choice-path", "{path.display()}" }
                    }
                    div { class: "profiles-panel__open-choice-name",
                        TextInput {
                            value: pending_open_name_read,
                            size: InputSize::Sm,
                            placeholder: "Library name".to_owned(),
                            oninput: move |evt: FormEvent| pending_open_name.set(evt.value()),
                        }
                    }
                    div { class: "profiles-panel__open-choice-actions",
                        Button { variant: ButtonVariant::Primary, size: ButtonSize::Sm, onclick: load_once_click, "Load once" }
                        Button { variant: ButtonVariant::Primary, size: ButtonSize::Sm, onclick: add_to_library_click, "Add to library" }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: cancel_open, "Cancel" }
                    }
                }
            }
            div { class: "profiles-panel__filter",
                TextInput {
                    value: filter_read,
                    size: InputSize::Sm,
                    placeholder: "Filter profiles".to_owned(),
                    oninput: move |evt: FormEvent| filter.set(evt.value()),
                }
            }
            div { class: "profiles-panel__body",
                if has_profile {
                    ProfileLibrary {
                        rows: meta.profile_rows.clone(),
                        active_id: meta.active_profile_id.clone().unwrap_or_default(),
                        filter: filter.read().clone(),
                    }
                } else {
                    NoProfileBar { on_new_profile: open_new_profile, on_open_file: open_file_click }
                }
            }
            if has_profile {
                SnapshotDrawer {
                    active_profile_name,
                    rows: meta.snapshot_rows.clone(),
                    open: true,
                }
            } else {
                footer { class: "profiles-panel__snapshot-toggle",
                    "Load a profile to view snapshots ({snapshot_count})"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
