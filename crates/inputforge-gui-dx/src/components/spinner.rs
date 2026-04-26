use dioxus::prelude::*;

use super::merge_class;

/// Dedicated size enum so a future `SpinnerSize::Xl` can land without
/// polluting `text_input::InputSize`. Conceptually unrelated to input sizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SpinnerSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl SpinnerSize {
    #[must_use]
    pub(crate) fn class(self) -> &'static str {
        match self {
            SpinnerSize::Sm => "if-spinner--sm",
            SpinnerSize::Md => "if-spinner--md",
            SpinnerSize::Lg => "if-spinner--lg",
        }
    }
}

#[component]
pub fn Spinner(
    #[props(default = SpinnerSize::Md)] size: SpinnerSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-spinner", size.class(), class.as_deref());
    rsx! { div { class: "{combined}", "aria-busy": "true", role: "status" } }
}
