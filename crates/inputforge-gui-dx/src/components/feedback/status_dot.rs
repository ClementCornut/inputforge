//! Compact decorative or explicitly named status indicators.

use dioxus::prelude::*;

use super::badge::BadgeVariant;
use crate::components::merge_class;

/// A status dot. Omit `label` when adjacent text already names the state.
#[component]
pub fn StatusDot(
    #[props(default = BadgeVariant::Neutral)] variant: BadgeVariant,
    #[props(default)] label: Option<String>,
    #[props(default)] hollow: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let tone = match variant {
        BadgeVariant::Neutral => "if-status-dot--neutral",
        BadgeVariant::Info => "if-status-dot--info",
        BadgeVariant::Success => "if-status-dot--success",
        BadgeVariant::Warning => "if-status-dot--warning",
        BadgeVariant::Error => "if-status-dot--error",
    };
    let variants = if hollow {
        format!("{tone} if-status-dot--hollow")
    } else {
        tone.to_owned()
    };
    let combined = merge_class("if-status-dot", &variants, class.as_deref());
    let decorative = label.is_none();
    rsx! {
        span {
            class: combined,
            role: (!decorative).then_some("img"),
            "aria-label": label,
            "aria-hidden": decorative.then_some("true"),
        }
    }
}
