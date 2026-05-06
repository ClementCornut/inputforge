use dioxus::prelude::*;

use crate::context::{ProfileRowOrigin, ProfileRowView};
use crate::frame::profiles::projection::project_profile_rows;

#[component]
pub(crate) fn ProfileLibrary(rows: Vec<ProfileRowView>, active_id: String) -> Element {
    let projected = project_profile_rows(&rows, &active_id, "");

    rsx! {
        div { class: "profiles-panel__library",
            for row in projected {
                article {
                    class: if row.is_active { "profile-row profile-row--active" } else { "profile-row" },
                    "data-profile-id": "{row.id}",
                    div { class: "profile-row__main",
                        strong { "{row.name}" }
                        span { class: "profile-row__path", "{row.path_label}" }
                    }
                    div { class: "profile-row__actions",
                        if row.can_open { button { class: "button", "Open" } }
                        if row.can_add_to_library { button { class: "button", "Add to library" } }
                        if row.can_snapshot_now { button { class: "button", "Snapshot now" } }
                        if row.can_rename { button { class: "button", "Rename" } }
                        if row.can_duplicate { button { class: "button", "Duplicate" } }
                        if row.can_reveal { button { class: "button", "Reveal" } }
                        if row.can_delete { button { class: "button button--danger", "Delete" } }
                    }
                    if matches!(row.origin, ProfileRowOrigin::External) {
                        span { class: "profile-row__badge", "External" }
                    }
                }
            }
        }
    }
}
