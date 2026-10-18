//! Drag-handle markup and app-supplied source event wiring.
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
    #[props(default = true)] draggable: bool,
) -> Element {
    let view = (state.bindings.handle)(crate::sortable::HandleProps {
        state,
        index,
        group,
        context: context.clone(),
        draggable,
    });
    let handle = view.id;
    let owner = format!("{:?}", state.drag_from.id());
    let ondragstart = view.start;
    let ondragend = view.end;
    let draggable_str = if draggable { "true" } else { "false" };

    rsx! {
        span {
            class: "if-sortable-handle",
            draggable: "false",
            "data-sortable-enabled": "{draggable_str}",
            "data-sortable-owner": "{owner}",
            "data-sortable-handle": "{handle}",
            "data-sortable-context": "{context}",
            "onpointerdown": view.pointer,
            "onclick": view.click,
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
