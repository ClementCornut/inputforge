use dioxus::prelude::*;

mod logic;

#[component]
pub(crate) fn ToolsCluster() -> Element {
    rsx! { span { class: "if-tools-cluster if-tools-cluster--placeholder", "Tools" } }
}
