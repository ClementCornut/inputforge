//! No-vJoy empty state. Renders when `AppState.virtual_devices` is
//! empty (no vJoy devices configured) or when no profile is loaded.
//!
//! `caption` lets the panel customize the helper text; the title is
//! invariant. The icon uses `IconKind::Info` (the closest neutral
//! glyph in the project's icon set; no `CircleSlash` exists yet).

use dioxus::prelude::*;

use crate::components::Icon;
use crate::icons::Icon as IconKind;

#[component]
pub(super) fn NoVjoyEmptyState(
    #[props(default = "Configure outputs in vJoyConf, then reopen.".to_owned())] caption: String,
    #[props(default = "No vJoy devices configured".to_owned())] title: String,
) -> Element {
    rsx! {
        div { class: "if-bulk-map__empty",
            div { class: "if-bulk-map__empty-icon", "aria-hidden": "true",
                Icon { name: IconKind::Info }
            }
            h3 { class: "if-bulk-map__empty-title", "{title}" }
            p { class: "if-bulk-map__empty-caption", "{caption}" }
        }
    }
}
