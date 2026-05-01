//! Shared state types for the sortable primitive.
//!
//! `SortableState` is the four-signal context shared between
//! `use_sortable_state`, `use_sortable_item`, `SortableHandle`, and
//! `SortableLiveRegion`. Consumers create one per list (rail) and pass
//! it down to every row.

use dioxus::prelude::*;

/// Which side of the hovered row the insertion bar paints on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortableSide {
    /// Bar above the hovered row -- drop lands at `hovered`.
    Before,
    /// Bar below the hovered row -- drop lands at `hovered + 1`.
    After,
}

/// In-flight drop indicator. Set by `use_sortable_item.ondragover` of the
/// hovered row, cleared on `ondragleave` (cursor leaves the row) or
/// `ondragend` / `ondrop` (drag operation ends). Valid and invalid
/// indicators share the same lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DropTarget {
    /// Group-local index of the hovered row.
    pub index: usize,
    /// Discriminator of the hovered row's group. Required because group-
    /// local `index` values are not unique across groups (e.g., Axes idx
    /// 0 and Buttons idx 0 both exist): a single `index` filter would
    /// paint the indicator on every row whose subgroup-index matches,
    /// across groups. Consumers must filter on `(index, group)` together.
    pub group: u32,
    pub side: SortableSide,
    /// `true` when the source's group does not match the target's group
    /// (per the consumer's `validate_drop`). The indicator paints in
    /// `--color-error` and stays visible while the cursor remains on
    /// the invalid row; clears on `dragleave` / `dragend` / `drop`.
    pub invalid: bool,
}

/// Shared state for one sortable list. Created via `use_sortable_state`.
///
/// All four signals are `Copy` (Dioxus 0.7 `Signal<T>`), so the struct
/// itself is `Copy`/`Clone` and can be passed by value through props.
/// `PartialEq` defers to per-signal pointer equality, which is what the
/// `#[component]` macro needs for prop-change detection.
#[allow(
    missing_debug_implementations,
    reason = "dioxus Signal<T> does not implement Debug"
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SortableState {
    /// Source row index, set on dragstart, cleared on dragend / drop.
    /// `None` means no drag is in flight.
    pub drag_from: Signal<Option<usize>>,
    /// Source row's group discriminator. Same lifetime as `drag_from`.
    pub drag_group: Signal<Option<u32>>,
    /// Currently-hovered drop indicator. Set on dragover of the target
    /// row, cleared on dragleave / drop / 200ms invalid timer.
    pub drop_target: Signal<Option<DropTarget>>,
    /// AT live-region content. Consumer writes the formatted reorder
    /// announcement here at every reorder dispatch site so AT users
    /// hear the outcome of every reorder path.
    pub live_announcement: Signal<String>,
}

/// Allocate a fresh `SortableState`. Mounts four `Signal`s on the calling
/// component, returns the bundle by value. Idiomatic usage:
///
/// ```ignore
/// let sortable = use_sortable_state();
/// rsx! {
///     for (idx, item) in items.iter().enumerate() {
///         Row { sortable, index: idx, /* ... */ }
///     }
///     SortableLiveRegion { state: sortable }
/// }
/// ```
pub fn use_sortable_state() -> SortableState {
    SortableState {
        drag_from: use_signal(|| None),
        drag_group: use_signal(|| None),
        drop_target: use_signal(|| None),
        live_announcement: use_signal(String::new),
    }
}

/// Translate a (source, hovered, side) tuple into the post-move index of
/// the source row in its group. Lifted from the official Dioxus
/// `DragAndDropList` primitive.
///
/// Semantics:
///   - `Before` puts the source at slot `hovered`.
///   - `After` puts the source at slot `hovered + 1`.
///   - When `from < slot` the source's removal shifts every later index
///     left by 1, so subtract 1 from `slot`.
///
/// # Examples
///
/// ```ignore
/// // moving down: source 0 -> above target 2 -> lands at 1
/// assert_eq!(resolve_drop_index(0, 2, SortableSide::Before), 1);
/// // moving down: source 0 -> below target 2 -> lands at 2
/// assert_eq!(resolve_drop_index(0, 2, SortableSide::After), 2);
/// // moving up:   source 3 -> above target 1 -> lands at 1
/// assert_eq!(resolve_drop_index(3, 1, SortableSide::Before), 1);
/// // moving up:   source 3 -> below target 1 -> lands at 2
/// assert_eq!(resolve_drop_index(3, 1, SortableSide::After), 2);
/// ```
#[must_use]
pub fn resolve_drop_index(from: usize, hovered: usize, side: SortableSide) -> usize {
    let slot = match side {
        SortableSide::Before => hovered,
        SortableSide::After => hovered + 1,
    };
    if from < slot { slot - 1 } else { slot }
}

#[cfg(test)]
mod tests {
    use super::{SortableSide, resolve_drop_index};

    // Tests lifted verbatim from the official `DragAndDropList` reference.

    #[test]
    fn move_down_before_target() {
        // source 0 -> above target 2 -> lands at 1
        assert_eq!(resolve_drop_index(0, 2, SortableSide::Before), 1);
    }

    #[test]
    fn move_down_after_target() {
        // source 0 -> below target 2 -> lands at 2
        assert_eq!(resolve_drop_index(0, 2, SortableSide::After), 2);
    }

    #[test]
    fn move_up_before_target() {
        // source 3 -> above target 1 -> lands at 1
        assert_eq!(resolve_drop_index(3, 1, SortableSide::Before), 1);
    }

    #[test]
    fn move_up_after_target() {
        // source 3 -> below target 1 -> lands at 2
        assert_eq!(resolve_drop_index(3, 1, SortableSide::After), 2);
    }

    #[test]
    fn same_index_before_is_noop_slot() {
        // hovered == from with side Before -> slot is `from`, from < slot
        // is false, so slot stays at `from` (a no-op move).
        assert_eq!(resolve_drop_index(2, 2, SortableSide::Before), 2);
    }

    #[test]
    fn same_index_after_lands_one_past() {
        // hovered == from with side After -> slot is `from + 1`, from <
        // slot is true, subtract 1 -> back to `from`.
        assert_eq!(resolve_drop_index(2, 2, SortableSide::After), 2);
    }
}
