//! Per-row drag/drop event-handler bundle.
//!
//! Consumers attach the returned handlers to their row container; the
//! handle (with `ondragstart`) is rendered separately via
//! `SortableHandle`. The cursor-Y midpoint pattern + invalid-flash
//! timer are lifted from the official Dioxus `DragAndDropList`.

use std::rc::Rc;

use dioxus::prelude::*;

use super::state::{DropTarget, SortableSide, SortableState, resolve_drop_index};

/// Configuration for `use_sortable_item`. One instance per row.
///
/// `validate_drop` is `Option<fn>` (not a closure) on purpose -- a
/// function pointer is `Copy + 'static`, which means the hook can stash
/// it inside async drop-target timers without lifetime contortions.
/// `None` is treated as "always allow" (cross-group drops are fine).
#[allow(
    missing_debug_implementations,
    reason = "dioxus Signal<T>, EventHandler<T>, and the user-supplied F closure do not implement Debug"
)]
pub struct SortableItemConfig<F: FnMut(usize, SortableSide) + 'static> {
    pub state: SortableState,
    pub index: usize,
    pub group: u32,
    pub group_len: usize,
    pub item_ref: Signal<Option<Rc<MountedData>>>,
    pub validate_drop: Option<fn(u32, u32) -> bool>,
    pub on_drop: F,
}

/// Event handlers returned by `use_sortable_item`. Spread these onto
/// the consumer's row container element. `ondragstart` is NOT exposed
/// here -- it lives on the `SortableHandle` so only the handle initiates
/// drags.
#[allow(
    dead_code,
    reason = "Public API surface; consumers are introduced in Phase A migration + F9."
)]
#[allow(
    missing_debug_implementations,
    reason = "dioxus EventHandler<T> does not implement Debug"
)]
pub struct SortableItemHandlers {
    pub ondragover: EventHandler<Event<DragData>>,
    pub ondragleave: EventHandler<Event<DragData>>,
    pub ondragend: EventHandler<Event<DragData>>,
    pub ondrop: EventHandler<Event<DragData>>,
}

/// Build per-row drag/drop handlers backed by the shared `SortableState`.
///
/// Behavior contract (preserved from F8):
/// * `ondragover`: `prevent_default()`, then awaits the row's bounding
///   client rect to compute `Before` / `After` from the cursor's Y
///   midpoint. Cross-group drops set an invalid `DropTarget` and spawn
///   a 200ms timer to clear it.
/// * `ondragleave`: clears `drop_target` only when it currently points at
///   THIS row AND is non-invalid (invalid clears via the timer).
/// * `ondragend`: clears `drag_from`, `drag_group`, and any non-invalid
///   `drop_target`.
/// * `ondrop`: `prevent_default()`, validator gate, computes final index
///   via `resolve_drop_index`, calls `on_drop(to, side)`, clears state.
///   The `on_drop` closure runs on the **target** row (the row whose
///   `ondrop` fired), not the source. Consumers that need the source
///   row's identity must read `state.drag_from` / `state.drag_group`
///   inside the callback; both are still populated and are cleared by
///   the primitive only after the closure returns.
#[allow(
    clippy::too_many_lines,
    reason = "All four handlers (over/leave/end/drop) live in one hook by design \
              so they share captured state without an extra wrapper struct."
)]
pub fn use_sortable_item<F>(config: SortableItemConfig<F>) -> SortableItemHandlers
where
    F: FnMut(usize, SortableSide) + 'static,
{
    let SortableItemConfig {
        state,
        index,
        group,
        group_len: _group_len,
        item_ref,
        validate_drop,
        on_drop,
    } = config;

    // Hoist the on_drop closure into a Signal so we can move a Copy handle
    // into the ondrop EventHandler without cloning the closure (which we
    // can't, since `F` is only `FnMut`).
    let mut on_drop_sig: Signal<Box<dyn FnMut(usize, SortableSide) + 'static>> =
        use_signal(|| Box::new(on_drop) as Box<_>);

    let mut drag_from = state.drag_from;
    let mut drag_group = state.drag_group;
    let mut drop_target = state.drop_target;

    let ondragover = EventHandler::new(move |evt: Event<DragData>| {
        // Always preventDefault so the browser flips the cursor to "move"
        // and accepts a subsequent drop. We still return early without
        // setting an indicator if there's no in-flight drag, but
        // preventing default is harmless in that case.
        evt.prevent_default();
        let Some(_src_idx) = *drag_from.peek() else {
            return;
        };
        let Some(src_group) = *drag_group.peek() else {
            return;
        };

        let invalid = validate_drop.is_some_and(|f| !f(src_group, group));
        let cursor_y = evt.client_coordinates().y;
        let target_ref = item_ref.peek().clone();
        spawn(async move {
            // `Before` / `After` from the row's mid-line. If the rect read
            // fails (e.g. element unmounted between dispatch and await),
            // skip the indicator update -- the next mousemove will retry.
            let side = if let Some(md) = target_ref {
                if let Ok(rect) = md.get_client_rect().await {
                    let mid_y = rect.origin.y + rect.size.height / 2.0;
                    if cursor_y < mid_y {
                        SortableSide::Before
                    } else {
                        SortableSide::After
                    }
                } else {
                    return;
                }
            } else {
                return;
            };

            drop_target.set(Some(DropTarget {
                index,
                group,
                side,
                invalid,
            }));
            // No self-clearing timer for invalid drops: `dragover` fires
            // repeatedly while the cursor sits on the row, so any timer
            // would race the next dragover and produce a visible
            // on/off flicker. The indicator is cleared on `dragleave`
            // (cursor moves off the row) or on `dragend`/`drop` (drag
            // operation ends), the same lifecycle as a valid indicator.
        });
    });

    let ondragleave = EventHandler::new(move |_evt: Event<DragData>| {
        // Match on `(index, group)`: `index` alone is not unique across
        // groups, so a leave on Axes row 1 should not clear an indicator
        // that's currently on Buttons row 1. Clears valid AND invalid
        // indicators: there is no longer an auto-clear timer for
        // invalid drops, so this handler is the canonical "cursor left
        // the row" signal for both kinds.
        if drop_target
            .peek()
            .as_ref()
            .is_some_and(|d| d.index == index && d.group == group)
        {
            drop_target.set(None);
        }
    });

    let ondragend = EventHandler::new(move |_evt: Event<DragData>| {
        drag_from.set(None);
        drag_group.set(None);
        // Clear any stale indicator on drag end. Covers both valid and
        // invalid because the invalid timer no longer exists; if the
        // user releases the mouse outside any drop target, an invalid
        // indicator on the last-hovered row would otherwise stick.
        if drop_target.peek().is_some() {
            drop_target.set(None);
        }
    });

    let ondrop = EventHandler::new(move |evt: Event<DragData>| {
        evt.prevent_default();
        let Some(from) = *drag_from.peek() else {
            return;
        };
        let Some(src_group) = *drag_group.peek() else {
            return;
        };
        // Validator gate (defense in depth: cross-group dragover does
        // call preventDefault per the official primitive's pattern, so
        // an invalid drop event CAN reach here).
        if validate_drop.is_some_and(|f| !f(src_group, group)) {
            drag_from.set(None);
            drag_group.set(None);
            drop_target.set(None);
            return;
        }
        // Insertion side comes from the indicator that ondragover was
        // last writing. Default to After if the indicator is missing
        // (last-mouse-move lost between dispatches).
        let side = drop_target
            .peek()
            .as_ref()
            .map_or(SortableSide::After, |d| d.side);
        let to = resolve_drop_index(from, index, side);
        let mut writer = on_drop_sig.write();
        (writer)(to, side);
        drop(writer);
        drag_from.set(None);
        drag_group.set(None);
        drop_target.set(None);
    });

    SortableItemHandlers {
        ondragover,
        ondragleave,
        ondragend,
        ondrop,
    }
}
