//! F15 settings panel. See docs/superpowers/specs/2026-05-09-f15-settings-ui-design.md.

mod field_row;
mod prune_confirm;
mod section;
mod snapshots_section;

#[allow(
    unused_imports,
    reason = "Forward-exported for Task 12+ consumers (SettingsPanel root)."
)]
pub(crate) use snapshots_section::SnapshotsSection;

use dioxus::prelude::*;

#[component]
pub(crate) fn SettingsPanel() -> Element {
    rsx! {
        div { class: "if-settings-panel-stub", "Settings panel (stub)" }
    }
}
