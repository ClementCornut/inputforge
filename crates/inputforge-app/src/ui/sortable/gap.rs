//! Gap eligibility, adjacent suppression and exactly-once drop cleanup.
use super::owner;
use dioxus::prelude::*;
use inputforge_gui_dx::sortable::{DropTarget, GapProps, GapView};
pub(super) fn use_gap<G: 'static + Clone + PartialEq>(
    props: GapProps<G>,
    validate: fn(usize, &G, &G) -> bool,
) -> GapView {
    let GapProps {
        mut state,
        gap_index,
        group,
        on_drop,
    } = props;
    let marker = state.drop_target.read();
    let mut class = String::from("if-sortable-gap");
    if let Some(target) = marker
        .as_ref()
        .filter(|t| t.gap_index == gap_index && t.group == group)
    {
        class.push_str(if target.invalid {
            " if-sortable-gap--target-invalid"
        } else {
            " if-sortable-gap--target"
        });
    }
    drop(marker);
    let over_group = group.clone();
    let leave_group = group.clone();
    let busy = use_signal(|| false);
    GapView {
        class,
        over: use_callback(move |event: DragEvent| {
            event.prevent_default();
            if *state.closed.peek() {
                return;
            }
            let Some(index) = *state.drag_from.peek() else {
                return;
            };
            let Some(source) = state.drag_group.peek().clone() else {
                return;
            };
            if source == over_group && adjacent(index, gap_index) {
                state.drop_target.set(None);
                state
                    .live_announcement
                    .set("Drop position same as source".into());
                return;
            }
            state.drop_target.set(Some(DropTarget {
                gap_index,
                group: over_group.clone(),
                invalid: !validate(index, &source, &over_group),
            }));
        }),
        leave: use_callback(move |_| {
            if !*state.closed.peek()
                && state
                    .drop_target
                    .peek()
                    .as_ref()
                    .is_some_and(|t| t.gap_index == gap_index && t.group == leave_group)
            {
                state.drop_target.set(None);
            }
        }),
        end: use_callback(move |_| {
            if !*state.closed.peek() {
                owner::clear(state);
            }
        }),
        drop: use_callback(move |event: DragEvent| {
            event.prevent_default();
            if *state.closed.peek() || *busy.peek() {
                return;
            }
            let Some(index) = *state.drag_from.peek() else {
                return;
            };
            let Some(source) = state.drag_group.peek().clone() else {
                return;
            };
            if validate(index, &source, &group) && !(source == group && adjacent(index, gap_index))
            {
                let mut busy = busy;
                busy.set(true);
                on_drop.call(gap_index);
                busy.set(false);
            }
            owner::clear(state);
        }),
    }
}
fn adjacent(source: usize, gap: usize) -> bool {
    gap == source || source.checked_add(1) == Some(gap)
}
