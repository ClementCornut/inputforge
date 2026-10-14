//! Drag-handle component. Renders a 6-dot grip SVG and owns the
//! `ondragstart` handler for the row.
//!
//! Only the handle starts a pointer gesture; the row body keeps click-to-select
//! semantics. The gesture forwards drag events to the existing Rust handlers.

use dioxus::prelude::*;

use super::state::SortableState;

/// Hover-revealed drag-handle. Wraps a 6-dot grip SVG (2x3 grid).
///
/// Browser pointer capture permits wheel scrolling and forwards hover/drop
/// events to the existing gap handlers. Native HTML dragging is disabled so
/// the browser does not suppress normal input for the duration of a gesture.
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
pub fn SortableHandle<G: 'static + Clone + PartialEq>(
    state: SortableState<G>,
    index: usize,
    group: G,
    /// Prepared identity/revision of the source content; changing it cancels a gesture.
    context: String,
    #[allow(
        unused_variables,
        reason = "group_len is part of the public API; reserved for consumer parity \
                  even though the primitive itself doesn't read it at dragstart."
    )]
    group_len: usize,
    #[props(default = true)] draggable: bool,
) -> Element {
    let handle = super::interaction::use_handle();
    let owner = format!("{:?}", state.drag_from.id());
    let mut drag_from = state.drag_from;
    let mut drag_group = state.drag_group;
    let mut drop_target = state.drop_target;
    let mut active = use_signal(|| false);
    use_drop(move || {
        if *active.peek() {
            drag_from.set(None);
            drag_group.set(None);
            drop_target.set(None);
        }
    });

    let ondragstart = move |_evt: Event<DragData>| {
        active.set(true);
        drag_from.set(Some(index));
        // Clone the group into the signal. For Copy types (e.g. `u32`)
        // this is a zero-cost bitwise copy; for non-Copy types (e.g.
        // `StageId`) it performs one allocation at dragstart.
        drag_group.set(Some(group.clone()));
    };

    // Every completed or cancelled pointer gesture forwards dragend to its source.
    let ondragend = move |_evt: Event<DragData>| {
        active.set(false);
        drag_from.set(None);
        drag_group.set(None);
        if drop_target.peek().is_some() {
            drop_target.set(None);
        }
    };

    let draggable_str = if draggable { "true" } else { "false" };

    rsx! {
        span {
            class: "if-sortable-handle",
            draggable: "false",
            "data-sortable-enabled": "{draggable_str}",
            "data-sortable-owner": "{owner}",
            "data-sortable-handle": "{handle}",
            "data-sortable-context": "{context}",
            "onpointerdown": "window.__ifSortableStart?.(event);",
            "onclick": "if(this.__ifSortableMoved){{this.__ifSortableMoved=false;event.preventDefault();event.stopPropagation();}}",
            ondragstart,
            ondragend,
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
