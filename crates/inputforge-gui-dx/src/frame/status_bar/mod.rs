mod logic;

use dioxus::prelude::*;

#[component]
pub(crate) fn StatusBar() -> Element {
    rsx! { div { class: "if-frame-status-bar", "StatusBar — Task 19" } }
}
