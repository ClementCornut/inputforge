//! Drop-gap component. Renders an inter-row drop zone that owns its
//! own DnD event handlers and writes a single `DropTarget` to the
//! shared `SortableState` when the cursor enters it during a drag.

#![allow(
    unpredictable_function_pointer_comparisons,
    reason = "The `#[component]` macro auto-derives PartialEq on the props struct, \
              which compares `validate_drop` (an `Option<fn(&G, &G) -> bool>`) by \
              fn-pointer address. The comparison is for prop-change detection only; \
              equal-or-not is fine, address aliasing is benign here. Module-scope \
              allow because the lint fires from the macro expansion before any \
              fn-level allow takes effect."
)]
//!
//! A list of N rows has N+1 gaps: one before each row plus one trailing
//! gap after the last row. The gap's `gap_index` IS the destination slot
//! index; no `Before/After` enum, no rect midpoint computation, no async
//! `get_client_rect()` race. The previous primitive's row-half model
//! (`use_sortable_item`) had three observed bugs in tall-row consumers
//! (double bars between rows separated by flex `gap`, async ondragover
//! resolution races, source-adjacent self-drop ambiguity); the gap
//! model eliminates all three by construction.
//!
//! Source-adjacent suppression: when dragging row N in group G, gaps at
//! `gap_index == N` and `gap_index == N + 1` of group G are silent
//! no-ops. They do not paint a bar, do not write `drop_target`, and do
//! not call `on_drop`. Mouse users see no bar (the visible signal that
//! a drop here would be a self-move). Keyboard / screen-reader users
//! receive an AT announcement via `state.live_announcement` so the
//! suppression is legible without the visible cue.

use dioxus::prelude::*;

use super::state::{DropTarget, SortableState};

/// Returns `true` when the given gap is directly above or below the
/// drag-source row. A drop on either of these gaps would be a no-op
/// (move the row to its own current position), so the gap suppresses
/// its bar and short-circuits its drop handler.
///
/// Math: the source at index N occupies the slot between gap N and
/// gap N+1. Either gap, if used as the destination, would land the
/// row in its current position.
#[must_use]
pub(crate) fn is_source_adjacent(src_idx: usize, gap_index: usize) -> bool {
    gap_index == src_idx || gap_index == src_idx + 1
}

/// Inter-row drop zone. Mount one of these before row 0, between every
/// pair of rows, and after the last row of every sortable group. The
/// `gap_index` is the destination slot; for a group of N rows, valid
/// indices are `0..=N`.
///
/// `validate_drop` matches `SortableHandle`'s F8 contract: a function
/// pointer (not a closure) so the gap can stash it across captures
/// without lifetime contortions. `None` means "always allow"
/// (cross-group drops are fine).
///
/// `on_drop` receives only the destination `gap_index`. The source's
/// identity comes from `state.drag_from` / `state.drag_group`, which
/// are still populated when the closure runs and are cleared by the
/// gap after the closure returns.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub fn SortableGap<G: 'static + Clone + PartialEq>(
    state: SortableState<G>,
    gap_index: usize,
    group: G,
    /// Optional cross-group gate. `fn(&G, &G) -> bool`, returns `true`
    /// when a drop from the first group onto the second group is
    /// valid. `None` allows every drop.
    validate_drop: Option<fn(&G, &G) -> bool>,
    on_drop: EventHandler<usize>,
) -> Element {
    let mut drag_from = state.drag_from;
    let mut drag_group = state.drag_group;
    let mut drop_target = state.drop_target;
    let mut live_writer = state.live_announcement;

    // Pre-clone `group` once per closure that needs it. For Copy types
    // this is a zero-cost bitwise copy; for non-Copy types like
    // `StageId` it performs one allocation per `SortableGap` instance,
    // not per event.
    let group_for_over = group.clone();
    let group_for_leave = group.clone();
    let group_for_drop_evt = group.clone();
    let group_for_class = group.clone();

    // Read the live drop_target to derive modifier classes for this gap.
    // The `(gap_index, group)` filter is required because gap indices
    // are not unique across groups: gap 0 of Axes and gap 0 of Buttons
    // would otherwise both paint the indicator.
    let drop_marker = drop_target.read();
    let (is_target, is_invalid) = drop_marker
        .as_ref()
        .filter(|d| d.gap_index == gap_index && d.group == group_for_class)
        .map_or((false, false), |d| (true, d.invalid));
    drop(drop_marker);

    let mut class = String::from("if-sortable-gap");
    match (is_target, is_invalid) {
        (true, false) => class.push_str(" if-sortable-gap--target"),
        (true, true) => class.push_str(" if-sortable-gap--target-invalid"),
        _ => {}
    }

    let ondragover = move |evt: Event<DragData>| {
        // Always preventDefault so the browser flips the cursor to "move"
        // and accepts a subsequent drop. Harmless when no drag is in
        // flight (`drag_from` is None and we early-return).
        evt.prevent_default();
        let Some(src_idx) = *drag_from.peek() else {
            return;
        };
        let Some(src_group) = drag_group.peek().clone() else {
            return;
        };
        let invalid = validate_drop.is_some_and(|f| !f(&src_group, &group_for_over));
        let same_group = src_group == group_for_over;

        // Source-adjacent suppression: do not paint the bar, do not
        // write drop_target. Write an AT announcement so keyboard /
        // screen-reader users hear what mouse users see (the absent
        // bar). Cleared on the next dragover into a non-adjacent gap.
        if same_group && is_source_adjacent(src_idx, gap_index) {
            live_writer.set("Drop position same as source".to_owned());
            return;
        }

        drop_target.set(Some(DropTarget {
            gap_index,
            group: group_for_over.clone(),
            invalid,
        }));
    };

    let ondragleave = move |_evt: Event<DragData>| {
        // Clear only when the indicator currently points at THIS gap.
        // A leave on Axes gap 1 should not clear an indicator pointing
        // at Buttons gap 1.
        if drop_target
            .peek()
            .as_ref()
            .is_some_and(|d| d.gap_index == gap_index && d.group == group_for_leave)
        {
            drop_target.set(None);
        }
    };

    let ondragend = move |_evt: Event<DragData>| {
        drag_from.set(None);
        drag_group.set(None);
        if drop_target.peek().is_some() {
            drop_target.set(None);
        }
    };

    let ondrop = move |evt: Event<DragData>| {
        evt.prevent_default();
        let Some(src_idx) = *drag_from.peek() else {
            return;
        };
        let Some(src_group) = drag_group.peek().clone() else {
            return;
        };
        // Validator gate: defense in depth. The gap's ondragover already
        // skips writing drop_target for invalid drops, but the browser
        // will still fire ondrop here if dragover called preventDefault
        // (which it always does, see above). Re-check the validator and
        // bail without dispatching on_drop.
        if validate_drop.is_some_and(|f| !f(&src_group, &group_for_drop_evt)) {
            drag_from.set(None);
            drag_group.set(None);
            drop_target.set(None);
            return;
        }
        // Source-adjacent suppression at the drop level too. The gap's
        // ondragover already skips writing drop_target for these gaps,
        // but a stray drop event can still land here (e.g. if the user
        // drops without ever moving the cursor over a non-adjacent gap).
        if src_group == group_for_drop_evt && is_source_adjacent(src_idx, gap_index) {
            drag_from.set(None);
            drag_group.set(None);
            drop_target.set(None);
            return;
        }
        on_drop.call(gap_index);
        drag_from.set(None);
        drag_group.set(None);
        drop_target.set(None);
    };

    rsx! {
        li {
            class: "{class}",
            role: "presentation",
            "aria-hidden": "true",
            ondragover,
            ondragleave,
            ondragend,
            ondrop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_source_adjacent;

    #[test]
    fn source_adjacent_above() {
        // The gap directly above the source row: gap_index == src_idx.
        assert!(is_source_adjacent(2, 2));
    }

    #[test]
    fn source_adjacent_below() {
        // The gap directly below the source row: gap_index == src_idx + 1.
        assert!(is_source_adjacent(2, 3));
    }

    #[test]
    fn source_not_adjacent_far_above() {
        assert!(!is_source_adjacent(2, 1));
    }

    #[test]
    fn source_not_adjacent_far_below() {
        assert!(!is_source_adjacent(2, 4));
    }

    #[test]
    fn source_at_zero_above_is_first_gap() {
        // Source at index 0, gap 0 is above it (still adjacent).
        assert!(is_source_adjacent(0, 0));
    }

    #[test]
    fn source_at_zero_below_is_second_gap() {
        // Source at index 0, gap 1 is below it (still adjacent).
        assert!(is_source_adjacent(0, 1));
    }
}
