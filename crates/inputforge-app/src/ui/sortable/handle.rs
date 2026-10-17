//! Source event interpretation and mounted handle identity.
use super::owner;
use dioxus::prelude::*;
use inputforge_gui_dx::sortable::{HandleProps, HandleView};
use std::{cell::RefCell, rc::Rc};
pub(super) fn use_handle<G: 'static + Clone + PartialEq>(props: HandleProps<G>) -> HandleView {
    let identity = use_signal(|| ());
    let id = format!("{:?}", identity.id());
    let mut state = props.state;
    let mut mounted = use_signal(|| true);
    let end_id = id.clone();
    let finish = use_callback(move |()| {
        if *mounted.peek() && state.handle_id.peek().as_ref() == Some(&end_id) {
            owner::clear(state);
        }
    });
    let unmount_id = id.clone();
    use_drop(move || {
        finish.call(());
        mounted.set(false);
        owner::cancel("handle", &unmount_id);
    });
    let observed = (
        props.index,
        props.group.clone(),
        props.context.clone(),
        props.draggable,
    );
    let previous = use_hook(|| Rc::new(RefCell::new(observed.clone())));
    if *previous.borrow() != observed {
        finish.call(());
        owner::cancel("handle", &id);
        *previous.borrow_mut() = observed;
    }
    let start_id = id.clone();
    HandleView {
        id,
        start: use_callback(move |_| {
            if !*mounted.peek() || *state.closed.peek() || !props.draggable {
                return;
            }
            state.drag_from.set(Some(props.index));
            state.drag_group.set(Some(props.group.clone()));
            state.handle_id.set(Some(start_id.clone()));
            state.drop_target.set(None);
        }),
        end: use_callback(move |_| finish.call(())),
        pointer: "window.__ifSortableStart?.(event);",
        click: "window.__ifSortableClick?.(event);",
    }
}
