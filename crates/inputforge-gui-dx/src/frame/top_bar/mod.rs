mod engine_pill;
pub(crate) mod mode_tabs;
mod primary_nav;
mod profile_name;
mod tools_cluster;

use dioxus::prelude::*;

use engine_pill::EnginePill;
pub(crate) use mode_tabs::{ModeDeleteDialog, ModeDeleteSignal, ModeTabs};
use primary_nav::PrimaryNav;
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
    rsx! {
        Stylesheet { href: TOP_BAR_CSS }
        div { class: "if-top-bar",
            EnginePill {}
            div { class: "if-top-bar__divider" }
            ProfileName {}
            PrimaryNav {}
            div { class: "if-top-bar__spacer" }
            ToolsCluster {}
        }
    }
}
