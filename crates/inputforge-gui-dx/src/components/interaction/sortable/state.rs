//! Typed app publications and hook wiring for sortable rendering.
use crate::sortable::Bindings;
#[doc(inline)]
pub use crate::sortable::{DropTarget, State as SortableState};
/// Mount the app's shared sortable behavior for one list.
#[must_use]
pub fn use_sortable_state<G: 'static + Clone + PartialEq>(
    bindings: Bindings<G>,
) -> SortableState<G> {
    (bindings.state)()
}
