//! Drag-handle component. Renders a 6-dot grip SVG and owns the
//! `ondragstart` handler for the row.
//!
//! Only the handle is `draggable=true`; the row body keeps click-to-
//! select semantics. Hover-only opacity is driven by the consumer's CSS
//! (Phase A hardcodes `.if-row:hover .if-sortable-handle`).

use dioxus::prelude::*;

use super::state::SortableState;

/// Hover-revealed drag-handle. Wraps a 6-dot grip SVG (2x3 grid).
///
/// The handle's `ondragstart` calls
/// `event.data_transfer().set_data("text/html", "")` -- this incantation
/// is required for Firefox/WebView2 to actually start a drag operation.
/// (Without it, the cursor stays as `no-drop` and the drop is never
/// fired.)
///
/// `group_len` is currently unused by the primitive itself but is kept
/// on the prop list so the consumer can pass it forward consistently
/// (the consumer needs it for live-region announcements, and routing
/// it through `SortableHandle` keeps "drag origin" data co-located).
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub fn SortableHandle(
    state: SortableState,
    index: usize,
    group: u32,
    #[allow(
        unused_variables,
        reason = "group_len is part of the public API; reserved for consumer parity \
                  even though the primitive itself doesn't read it at dragstart."
    )]
    group_len: usize,
    #[props(default = true)] draggable: bool,
) -> Element {
    let mut drag_from = state.drag_from;
    let mut drag_group = state.drag_group;

    let ondragstart = move |evt: Event<DragData>| {
        // Firefox/WebView2 fix: the source's dataTransfer must carry
        // some payload OR the drag is silently aborted. This is the
        // exact incantation from the official Dioxus DragAndDropList
        // primitive.
        let _ = evt.data_transfer().set_data("text/html", "");
        evt.data_transfer().set_effect_allowed("move");
        drag_from.set(Some(index));
        drag_group.set(Some(group));
    };

    let draggable_str = if draggable { "true" } else { "false" };

    rsx! {
        span {
            class: "if-sortable-handle",
            draggable: "{draggable_str}",
            ondragstart,
            "aria-hidden": "true",
            // 6-dot grip, 2x3 grid, 10x16 viewBox. Lifted verbatim from
            // F8's mapping_list/row.rs handle markup.
            svg {
                width: "10",
                height: "16",
                view_box: "0 0 10 16",
                fill: "currentColor",
                circle { cx: "2.5", cy: "3", r: "1.25" }
                circle { cx: "7.5", cy: "3", r: "1.25" }
                circle { cx: "2.5", cy: "8", r: "1.25" }
                circle { cx: "7.5", cy: "8", r: "1.25" }
                circle { cx: "2.5", cy: "13", r: "1.25" }
                circle { cx: "7.5", cy: "13", r: "1.25" }
            }
        }
    }
}
