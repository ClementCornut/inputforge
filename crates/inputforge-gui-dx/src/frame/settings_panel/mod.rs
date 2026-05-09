//! F15 settings panel. See docs/superpowers/specs/2026-05-09-f15-settings-ui-design.md.

use dioxus::prelude::*;

#[component]
pub(crate) fn SettingsPanel() -> Element {
    rsx! {
        div { class: "if-settings-panel-stub", "Settings panel (stub)" }
    }
}
