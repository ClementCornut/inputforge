use dioxus::prelude::*;

#[component]
pub(crate) fn ModeTabs() -> Element {
    rsx! { span { class: "if-mode-tabs if-mode-tabs--placeholder", "Tabs" } }
}
