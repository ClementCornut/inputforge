use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::profiles::no_profile::NoProfileBar;

pub(crate) mod no_profile;
pub(crate) mod projection;

const PROFILES_CSS: Asset = asset!("/assets/frame/profiles.css");

#[component]
pub(crate) fn ProfilesPanel() -> Element {
    let ctx = use_context::<AppContext>();
    let state = ctx.state.read();
    let snapshot_count = state.active_snapshot_rows.len();
    let has_profile = state.active_profile.is_some();
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
                    div { class: "profiles-panel__library", "Profile library" }
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
