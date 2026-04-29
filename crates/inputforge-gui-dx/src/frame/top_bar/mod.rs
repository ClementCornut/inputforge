use dioxus::prelude::*;

#[component]
pub(crate) fn TopBar() -> Element {
    rsx! { div { class: "if-top-bar", "TopBar — Task 21" } }
}
