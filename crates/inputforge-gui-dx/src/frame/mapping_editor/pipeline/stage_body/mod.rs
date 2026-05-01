// Rust guideline compliant 2026-05-01

//! Stage body dispatcher and per-variant body components.
//!
//! Task 20 ships only `header_right_slot`, the helper that returns the
//! Element rendered inside the `StageHeader`'s 32x32 chevron slot.
//! Default for all F9-owned variants: chevron-down SVG. F10/F11/F14
//! override their variants' branch in this helper to inject a preview
//! thumbnail. Body dispatcher and per-variant components land in Task 22+.

#![allow(
    dead_code,
    reason = "header_right_slot consumed by stage.rs; body dispatcher lands in Task 22+"
)]

use dioxus::prelude::*;

use inputforge_core::action::Action;

/// Returns the Element rendered inside the stage header's 32x32 chevron
/// slot. For all Task-20 variants the default is a chevron-down SVG that
/// rotates to indicate collapsed state. F10/F11/F14 will override specific
/// variant branches to inject a preview thumbnail.
pub(crate) fn header_right_slot(_action: &Action, expanded: bool) -> Element {
    let class = if expanded {
        "if-stage__chevron"
    } else {
        "if-stage__chevron if-stage__chevron--collapsed"
    };
    rsx! {
        svg {
            class: "{class}",
            width: "12",
            height: "12",
            view_box: "0 0 12 12",
            "aria-hidden": "true",
            path {
                d: "M2 4 L6 8 L10 4",
                stroke: "currentColor",
                stroke_width: "1.5",
                fill: "none",
            }
        }
    }
}
