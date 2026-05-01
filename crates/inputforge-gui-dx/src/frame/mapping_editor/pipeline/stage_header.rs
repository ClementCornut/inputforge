// Rust guideline compliant 2026-05-01

//! Stage header row: expand/collapse button with title, summary, and
//! chevron/thumbnail slot.
//!
//! Per spec lines 322-326 + 586. The header is a single full-width
//! `<button>` (keyboard-accessible per AC #21). The `right_slot` prop
//! renders the chevron SVG (default) or a preview thumbnail (F10/F11).
//!
//! **Adaptation note:** The F2 `IconButton` primitive does not accept
//! arbitrary children (it renders only its own `Icon` element). The
//! header layout requires title + summary + slot inside the same
//! interactive surface, so a plain `<button>` is used instead. This
//! preserves the invariant that the entire header row is one keyboard-
//! focusable element (Space/Enter toggle expand per AC #21) while
//! accommodating the real component API.

use dioxus::prelude::*;

use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn StageHeader(
    stage_id: StageId,
    title: String,
    summary: String,
    expanded: bool,
    /// Element rendered in the 32x32 chevron/thumbnail slot on the right.
    /// Default for all F9-owned variants: chevron-down SVG (from
    /// `stage_body::header_right_slot`). F10/F11 override with a preview
    /// thumbnail.
    right_slot: Element,
) -> Element {
    let editor = use_context::<EditorState>();
    let mut expanded_set = editor.expanded_stages;
    let stage_id_for_click = stage_id.clone();
    let controls_id = format!("if-stage-body-{}", format_stage_id(&stage_id));

    let onclick = move |_evt: MouseEvent| {
        let mut set = expanded_set.write();
        if !set.insert(stage_id_for_click.clone()) {
            set.remove(&stage_id_for_click);
        }
    };

    rsx! {
        button {
            r#type: "button",
            class: "if-stage__header",
            "aria-expanded": if expanded { "true" } else { "false" },
            "aria-controls": "{controls_id}",
            onclick,
            div { class: "if-stage__title", "{title}" }
            div { class: "if-stage__summary", "{summary}" }
            div {
                class: "if-stage__right-slot",
                "aria-hidden": "true",
                {right_slot}
            }
        }
    }
}

fn format_stage_id(id: &StageId) -> String {
    use crate::frame::mapping_editor::undo_log::StageIdSegment;
    id.0.iter()
        .map(|seg| match seg {
            StageIdSegment::Index(i) => format!("{i}"),
            StageIdSegment::IfTrue => "T".to_owned(),
            StageIdSegment::IfFalse => "F".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(".")
}
