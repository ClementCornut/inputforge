mod logic;

use dioxus::prelude::*;

#[component]
pub(crate) fn EnginePill() -> Element {
    rsx! { span { class: "if-engine-pill if-engine-pill--placeholder", "Pill" } }
}
