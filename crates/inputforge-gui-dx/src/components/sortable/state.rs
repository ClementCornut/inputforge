//! Shared state types for the sortable primitive.
//!
//! `SortableState` is the four-signal context shared between
//! `use_sortable_state`, `SortableGap`, `SortableHandle`, and
//! `SortableLiveRegion`. Consumers create one per list (rail) and pass
//! it down to every row and gap.
//!
//! The group discriminator type `G` is generic so the same primitive can
//! serve both F8 (which uses `u32` bucket IDs) and F9 (which uses a
//! `StageId` path that is `Clone` but not `Copy`).

use dioxus::prelude::*;

/// In-flight drop indicator. Set by the gap's `ondragover` of the
/// hovered drop slot, cleared on `ondragleave` (cursor leaves the gap)
/// or `ondragend` / `ondrop` (drag operation ends). Valid and invalid
/// indicators share the same lifecycle.
///
/// `(gap_index, group)` is the unique key. Two adjacent groups in the
/// same list (e.g. rail Axes + rail Buttons, or sibling pipelines in
/// nested Conditionals) each have their own gap-index space, so the
/// `group` field is required: gap_index 0 of Axes and gap_index 0 of
/// Buttons are different cells. Consumers MUST filter on the pair, not
/// on `gap_index` alone.
///
/// `gap_index` ranges over `[0, group_len]` inclusive: there is one gap
/// before each row plus one trailing gap after the last row, so a group
/// with N rows has N+1 gaps.
///
/// `Copy` is intentionally omitted: `G` is not required to be `Copy`
/// (e.g. `StageId` is a `Vec`-backed path that is `Clone` but not
/// `Copy`). Callers that need a fresh value should `.clone()`.
#[derive(Clone, Debug, PartialEq)]
pub struct DropTarget<G: 'static + Clone + PartialEq> {
    /// Slot index where the dragged source will land if dropped here.
    /// In `[0, group_len]` inclusive.
    pub gap_index: usize,
    /// Discriminator of the gap's group. Required because `gap_index`
    /// is not unique across groups: a single `gap_index` filter would
    /// paint the indicator on every gap whose index matches, across
    /// groups. Consumers must filter on `(gap_index, group)` together.
    pub group: G,
    /// `true` when the source's group does not satisfy the consumer's
    /// `validate_drop` against this gap's group. The indicator paints
    /// in `--color-error` and stays visible while the cursor remains on
    /// the invalid gap; clears on `dragleave` / `dragend` / `drop`.
    pub invalid: bool,
}

/// Shared state for one sortable list. Created via `use_sortable_state`.
///
/// All four signals are `Copy` (Dioxus 0.7 `Signal<T>`), so the struct
/// itself is `Copy`/`Clone` and can be passed by value through props.
/// `PartialEq` defers to per-signal pointer equality, which is what the
/// `#[component]` macro needs for prop-change detection.
///
/// `Copy` is implemented manually rather than via `#[derive(Copy)]`
/// because the derive infers a `G: Copy` bound from the generic
/// parameter even though every field uses `G` only through
/// `Signal<...>` (which is `Copy` regardless of `T`). The manual impl
/// expresses the actual requirement: any `G: 'static + Clone +
/// PartialEq` makes the bundle `Copy` because all four fields are
/// `Signal`, and `Signal<T>: Copy` holds for every `T: 'static`.
#[allow(
    missing_debug_implementations,
    reason = "dioxus Signal<T> does not implement Debug"
)]
pub struct SortableState<G: 'static + Clone + PartialEq> {
    /// Source row index, set on dragstart, cleared on dragend / drop.
    /// `None` means no drag is in flight.
    pub drag_from: Signal<Option<usize>>,
    /// Source row's group discriminator. Same lifetime as `drag_from`.
    pub drag_group: Signal<Option<G>>,
    /// Currently-hovered drop indicator. Set on dragover of the target
    /// gap, cleared on dragleave / drop / dragend.
    pub drop_target: Signal<Option<DropTarget<G>>>,
    /// AT live-region content. Consumer writes the formatted reorder
    /// announcement here at every reorder dispatch site so AT users
    /// hear the outcome of every reorder path.
    pub live_announcement: Signal<String>,
}

impl<G: 'static + Clone + PartialEq> Copy for SortableState<G> {}

#[allow(
    clippy::expl_impl_clone_on_copy,
    reason = "Manual Clone impl is required because `derive(Clone)` infers a \
              `G: Clone` bound from the generic even though every field is \
              `Signal`-wrapped. The struct is `Copy` so the impl body is `*self`."
)]
impl<G: 'static + Clone + PartialEq> Clone for SortableState<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: 'static + Clone + PartialEq> PartialEq for SortableState<G> {
    fn eq(&self, other: &Self) -> bool {
        self.drag_from == other.drag_from
            && self.drag_group == other.drag_group
            && self.drop_target == other.drop_target
            && self.live_announcement == other.live_announcement
    }
}

/// Allocate a fresh `SortableState`. Mounts four `Signal`s on the calling
/// component, returns the bundle by value. Idiomatic usage:
///
/// ```ignore
/// let sortable = use_sortable_state::<u32>();
/// rsx! {
///     SortableGap { state: sortable, gap_index: 0, /* ... */ }
///     for (idx, item) in items.iter().enumerate() {
///         Row { sortable, index: idx, /* ... */ }
///         SortableGap { state: sortable, gap_index: idx + 1, /* ... */ }
///     }
///     SortableLiveRegion { state: sortable }
/// }
/// ```
pub fn use_sortable_state<G: 'static + Clone + PartialEq>() -> SortableState<G> {
    SortableState {
        drag_from: use_signal(|| None),
        drag_group: use_signal(|| None),
        drop_target: use_signal(|| None),
        live_announcement: use_signal(String::new),
    }
}
