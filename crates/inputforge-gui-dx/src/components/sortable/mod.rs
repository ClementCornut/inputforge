//! Reusable sortable list primitive.
//!
//! Gap-drop-zone model: each list of N rows has N+1 explicit gap
//! elements (one before each row plus one trailing gap after the last
//! row). Each gap is a drop target with a single integer identity
//! (`gap_index`) that IS the destination slot. Rows own only the drag
//! source (the 6-dot grip + dragstart) and the dragging-source
//! modifier; rows do not own drop handling.
//!
//! Composition:
//!
//! * `use_sortable_state` -- one `SortableState` bundle per list
//! * `SortableHandle` -- the 6-dot grip with `ondragstart`, mounted
//!   inside each row
//! * `SortableGap` -- inter-row drop zone with synchronous
//!   ondragover / ondragleave / ondragend / ondrop, mounted between
//!   rows + leading + trailing per group
//! * `SortableLiveRegion` -- visually-hidden `aria-live="polite"` mount
//!
//! The `event.data_transfer().set_data("text/html", "")` incantation in
//! `SortableHandle.ondragstart` is required for Firefox / WebView2 to
//! actually start a native HTML5 drag operation.
//!
//! CSS lives in `/assets/components/sortable.css` and is mounted
//! alongside the rest of the design-system stylesheets in
//! `theme/mod.rs`.

#![allow(
    clippy::doc_markdown,
    reason = "Module doc references DnD as a domain term, not as code."
)]

mod gap;
mod handle;
mod live_region;
mod state;

pub use gap::SortableGap;
pub use handle::SortableHandle;
pub use live_region::SortableLiveRegion;
pub use state::{DropTarget, SortableState, use_sortable_state};
