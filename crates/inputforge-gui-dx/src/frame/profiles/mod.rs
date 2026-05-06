use dioxus::prelude::*;

use crate::context::AppContext;
use crate::context::MetaSnapshot;
use crate::frame::profiles::library::ProfileLibrary;
use crate::frame::profiles::no_profile::NoProfileBar;

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
    let state = ctx.state.read();
    let snapshot_count = state.active_snapshot_rows.len();
    let has_profile = state.active_profile.is_some();
    let meta = MetaSnapshot::from_state(&state);
    drop(state);

    rsx! {
        Stylesheet { href: PROFILES_CSS }
        section { class: "profiles-panel", "data-testid": "profile-library",
            header { class: "profiles-panel__header",
                h2 { "Profiles" }
                button { class: "button button--primary", "data-action": "new-profile", "+ New profile" }
                button { class: "button", "data-action": "open-profile", "Open file" }
            }
            div { class: "profiles-panel__body",
                if has_profile {
                    ProfileLibrary {
                        rows: meta.profile_rows.clone(),
                        active_id: meta.active_profile_id.clone().unwrap_or_default(),
                    }
                } else {
                    NoProfileBar {}
                }
            }
            footer { class: "profiles-panel__snapshot-toggle", "Snapshots - {snapshot_count}" }
        }
    }
}

#[cfg(test)]
mod tests;
