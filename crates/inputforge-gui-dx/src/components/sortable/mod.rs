//! Reusable sortable list primitive. Wraps the F8 mapping-list DnD
//! plumbing into a handful of drop-in pieces:
//!
//! * `use_sortable_state` -- one `SortableState` bundle per list
//! * `use_sortable_item` -- per-row event handlers
//! * `SortableHandle` -- the 6-dot grip with `ondragstart`
//! * `SortableLiveRegion` -- visually-hidden `aria-live="polite"` mount
//!
//! See `docs/superpowers/specs/2026-04-30-f8-mapping-list-design.md`
//! and the matching Phase A plan for the design rationale + behavior
//! contract. Behavior is preserved end-to-end from the F8
//! implementation, with one critical fix: the
//! `event.data_transfer().set_data("text/html", "")` incantation in
//! `SortableHandle.ondragstart` replaces the previous JS
//! `document::eval` bootstrap (Firefox/WebView2 require this for native
//! HTML5 drag-and-drop to actually start).
//!
//! CSS lives in `/assets/components/sortable.css` and is mounted
//! alongside the rest of the design-system stylesheets in
//! `theme/mod.rs`.

#![allow(
    clippy::doc_markdown,
    reason = "Module doc references DnD as a domain term, not as code."
)]

mod handle;
mod item;
mod live_region;
mod state;

pub use handle::SortableHandle;
pub use item::{SortableItemConfig, SortableItemHandlers, use_sortable_item};
pub use live_region::SortableLiveRegion;
pub use state::{DropTarget, SortableSide, SortableState, resolve_drop_index, use_sortable_state};
