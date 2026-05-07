use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::profiles::library::ProfileLibrary;
use crate::frame::profiles::new_profile_submode::NewProfileSubMode;
use crate::frame::profiles::no_profile::NoProfileBar;
use crate::frame::profiles::open_choice_submode::OpenChoiceSubMode;
use crate::frame::profiles::snapshot_drawer::SnapshotDrawer;
use crate::frame::view_state::{ProfilesPanelMode, ViewState};
use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;

pub(crate) mod actions;
pub(crate) mod delete_dialog;
pub(crate) mod library;
pub(crate) mod new_profile;
pub(crate) mod new_profile_submode;
pub(crate) mod no_profile;
pub(crate) mod open_choice_submode;
pub(crate) mod projection;
pub(crate) mod snapshot_drawer;

pub(crate) use delete_dialog::{ProfileDeleteDialog, ProfileDeleteSignal};

const PROFILES_CSS: Asset = asset!("/assets/frame/profiles.css");

/// Build a fresh Open-file click handler. Closures are not Clone,
/// but each call site (header button, no-profile bar) needs its own
/// copy, so we mint one per site rather than fight the Copy trait.
fn make_open_file_click(
    commands: std::sync::mpsc::Sender<EngineCommand>,
    mut view: ViewState,
) -> impl FnMut(MouseEvent) {
    move |_| {
        let commands = commands.clone();
        spawn(async move {
            if let Some(handle) = rfd::AsyncFileDialog::new()
                .add_filter("Profile (TOML)", &["toml"])
                .set_title("Open profile")
                .pick_file()
                .await
            {
                let path = handle.path().to_path_buf();
                // C3: if the picked file is already inside the
                // library directory, "Load once" vs "Add to
                // library" is moot; dispatch a normal LoadProfile
                // and stay on the library.
                let library_dir = AppSettings::profiles_dir();
                if path.starts_with(&library_dir) {
                    let _ = commands.send(EngineCommand::LoadProfile(path));
                    view.profiles_panel.write().mode = ProfilesPanelMode::Library;
                } else {
                    let suggested = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Imported")
                        .to_owned();
                    view.profiles_panel.write().mode = ProfilesPanelMode::OpenChoice {
                        path,
                        suggested_name: suggested,
                    };
                }
            }
        });
    }
}

#[component]
pub(crate) fn ProfilesPanel() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    // Panel-scoped destructive-confirm signal. The library row's
    // Delete onclick writes the target profile name here; the
    // sibling `ProfileDeleteDialog` reads it as its open-state +
    // dispatches `profile_delete_action(...).command` only when the
    // user confirms. Provided here so the trigger (inside
    // `ProfileLibrary`) and the dialog (sibling of the panel body)
    // share one source of truth.
    let delete_target = use_signal(|| Option::<String>::None);
    use_context_provider(|| ProfileDeleteSignal(delete_target));

    // Read from the reactive Signal published by the bridge polling
    // task at 60Hz so the panel re-renders on profile, snapshot, and
    // origin changes without forcing the user to switch sub-modes.
    let meta = ctx.meta.read().clone();
    let snapshot_count = meta.snapshot_rows.len();
    let has_profile = meta.profile_name.is_some();
    let active_profile_name = meta.profile_name.clone().unwrap_or_default();

    let panel_mode = view.profiles_panel.read().mode.clone();

    let mut view_for_new = view;
    let open_new_profile = move |_| {
        view_for_new.profiles_panel.write().mode = ProfilesPanelMode::NewProfile;
    };

    let library_open_file_click = make_open_file_click(ctx.commands.clone(), view);
    let no_profile_open_file_click = make_open_file_click(ctx.commands.clone(), view);

    rsx! {
        Stylesheet { href: PROFILES_CSS }
        section { class: "profiles-panel", "data-testid": "profile-library",
            // No panel-level <header>: the create/import affordances now
            // live inline next to the filter (Open file...) and as a
            // trailing row in the list (+ New profile). The slot tab
            // cluster already labels the surface "Profiles"; an in-panel
            // <h2> would restate it. The Devices panel already follows
            // the same header-less rule, locked by
            // `panel_slot/mod.rs::panel_header_omits_placeholder_caption`.
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
                                    on_new_profile: open_new_profile,
                                    on_open_file: library_open_file_click,
                                }
                            }
                        } else {
                            rsx! { NoProfileBar { on_new_profile: open_new_profile, on_open_file: no_profile_open_file_click } }
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
            // Destructive-confirm dialog. Lives at panel scope so it
            // shares the trigger's `ProfileDeleteSignal` context but
            // is not nested inside the row flex (a native `<dialog>`
            // renders on the browser's top layer regardless, so this
            // is purely organisational).
            ProfileDeleteDialog {}
        }
    }
}

#[cfg(test)]
mod tests;
