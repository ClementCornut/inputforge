//! Associated helper text and explicit workflow alerts.

use dioxus::prelude::*;

use crate::components::merge_class;

/// Presentation tone; announcing a workflow failure is an independent choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedbackTone {
    Helper,
    Error,
    Warning,
}

/// Normal-flow feedback. Associate controls through `aria_describedby` and `id`.
#[component]
pub fn InlineFeedback(
    #[props(default)] id: Option<String>,
    #[props(default = FeedbackTone::Helper)] tone: FeedbackTone,
    #[props(default)] alert: bool,
    /// Anchor floating feedback to a positioned inline-editor container.
    #[props(default)]
    floating: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let tone = match tone {
        FeedbackTone::Helper => "if-inline-feedback--helper",
        FeedbackTone::Error => "if-inline-feedback--error",
        FeedbackTone::Warning => "if-inline-feedback--warning",
    };
    let variants = if floating {
        format!("{tone} if-inline-feedback--floating")
    } else {
        tone.to_owned()
    };
    let combined = merge_class("if-inline-feedback", &variants, class.as_deref());
    rsx! {
        div { id, class: combined, role: alert.then_some("alert"), {children} }
    }
}
