use dioxus::prelude::*;

use crate::components::{Button, ButtonSize, ButtonVariant};
use crate::context::AppContext;
use crate::context::MetaSnapshot;
use crate::frame::profiles::library::ProfileLibrary;
use crate::frame::profiles::new_profile_submode::NewProfileSubMode;
use crate::frame::profiles::no_profile::NoProfileBar;
use crate::frame::profiles::open_choice_submode::OpenChoiceSubMode;
use crate::frame::profiles::snapshot_drawer::SnapshotDrawer;
use crate::frame::view_state::{ProfilesPanelMode, ViewState};

pub(crate) mod actions;
pub(crate) mod library;
pub(crate) mod new_profile;
pub(crate) mod new_profile_submode;
pub(crate) mod no_profile;
pub(crate) mod open_choice_submode;
pub(crate) mod projection;
pub(crate) mod snapshot_drawer;

const PROFILES_CSS: Asset = asset!("/assets/frame/profiles.css");

#[component]
pub(crate) fn ProfilesPanel() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

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

    let panel_mode = view.profiles_panel.read().mode.clone();

    let mut view_for_new = view;
    let open_new_profile = move |_| {
        view_for_new.profiles_panel.write().mode = ProfilesPanelMode::NewProfile;
    };

    let mut view_for_open = view;
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
                view_for_open.profiles_panel.write().mode = ProfilesPanelMode::OpenChoice {
                    path,
                    suggested_name: suggested,
                };
            }
        });
    };

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
            div { class: "profiles-panel__body",
                match panel_mode {
                    ProfilesPanelMode::NewProfile => rsx! { NewProfileSubMode {} },
                    ProfilesPanelMode::OpenChoice { path, suggested_name } => rsx! {
                        OpenChoiceSubMode { path, suggested_name }
                    },
                    ProfilesPanelMode::Library => {
                        if has_profile {
                            rsx! {
                                ProfileLibrary {
                                    rows: meta.profile_rows.clone(),
                                    active_id: meta.active_profile_id.clone().unwrap_or_default(),
                                }
                            }
                        } else {
                            rsx! { NoProfileBar { on_new_profile: open_new_profile, on_open_file: open_file_click } }
                        }
                    }
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
