//! AT live region for sortable lists. A single mount point per list.

use dioxus::prelude::*;

use super::state::SortableState;

/// Mounts a visually-hidden `aria-live="polite"` status region whose text
/// is driven by `state.live_announcement`. Consumers write to that signal
/// at every reorder dispatch site (drag-drop, context menu Move up/down,
/// keyboard Alt+Arrow) so AT users hear the outcome of every reorder
/// path.
///
/// The `if-sr-only` class is defined in `assets/global.css` (WCAG 2.1
/// "screen-reader-only" recipe).
#[component]
pub fn SortableLiveRegion(state: SortableState) -> Element {
    let text = state.live_announcement.read().clone();
    rsx! {
        span {
            class: "if-sr-only",
            role: "status",
            "aria-live": "polite",
            "{text}"
        }
    }
}
