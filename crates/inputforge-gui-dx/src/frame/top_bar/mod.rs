mod engine_pill;
mod mode_tabs;
mod profile_name;
mod tools_cluster;

use dioxus::prelude::*;

use engine_pill::EnginePill;
use mode_tabs::{ModeDeleteDialog, ModeDeleteSignal, ModeTabs};
use profile_name::ProfileName;
use tools_cluster::ToolsCluster;

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: TOP_BAR_CSS }"
)]
const TOP_BAR_CSS: Asset = asset!("/assets/frame/top_bar.css");

#[component]
pub(crate) fn TopBar() -> Element {
    tracing::trace!(target: "frame::render", region = "top_bar");
    // F4 delete-confirm target lives at the TopBar level so the
    // confirm dialog (`ModeDeleteDialog`) can render as a SIBLING of
    // `.if-top-bar` rather than nested inside it. Both `ModeTabs` (the
    // writer, on Delete keybind / context-menu Delete) and
    // `ModeDeleteDialog` (the reader, drives `dialog_open` and posts
    // the engine command) consume this through `ModeDeleteSignal`.
    let delete_target = use_signal(|| Option::<String>::None);
    use_context_provider(|| ModeDeleteSignal(delete_target));
    rsx! {
        Stylesheet { href: TOP_BAR_CSS }
        div { class: "if-top-bar",
            EnginePill {}
            div { class: "if-top-bar__divider" }
            ProfileName {}
            ModeTabs {}
            div { class: "if-top-bar__spacer" }
            ToolsCluster {}
        }
        ModeDeleteDialog {}
    }
}
