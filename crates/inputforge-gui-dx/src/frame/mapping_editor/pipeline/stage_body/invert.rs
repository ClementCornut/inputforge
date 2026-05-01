// Rust guideline compliant 2026-05-01

//! Invert body: descriptive caption only, no inputs.

use dioxus::prelude::*;

#[component]
pub(crate) fn InvertBody() -> Element {
    rsx! {
        div { class: "if-stage__body-caption",
            "Inverts the input value: x becomes -x."
        }
    }
}
