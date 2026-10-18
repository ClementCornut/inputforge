//! Caption treatment for compact instrument and status readouts.

use dioxus::prelude::*;

use crate::components::merge_class;

/// Monospaced caption text without automatic announcements or interaction.
#[component]
pub fn ReadoutCaption(#[props(default)] class: Option<String>, children: Element) -> Element {
    let combined = merge_class("if-readout-caption", "", class.as_deref());
    rsx! { span { class: combined, {children} } }
}
