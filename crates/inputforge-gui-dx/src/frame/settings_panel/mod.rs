//! F15 settings panel. See docs/superpowers/specs/2026-05-09-f15-settings-ui-design.md.

mod field_row;
mod section;

#[allow(
    unused_imports,
    reason = "Forward-exported for Task 11+ consumers (SnapshotsSection)."
)]
pub(crate) use field_row::SettingsFieldRow;
#[allow(
    unused_imports,
    reason = "Forward-exported for Task 11+ consumers (SnapshotsSection)."
)]
pub(crate) use section::SettingsSection;

use dioxus::prelude::*;

#[component]
pub(crate) fn SettingsPanel() -> Element {
    rsx! {
        div { class: "if-settings-panel-stub", "Settings panel (stub)" }
    }
}
