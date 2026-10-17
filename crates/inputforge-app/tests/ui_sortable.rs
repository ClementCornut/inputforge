//! Real app sortable callbacks reject stale owners, invalid groups and repeated drops.
use dioxus::prelude::*;
use inputforge_gui_dx::sortable::{GapProps, GapView, HandleProps, HandleView, State};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
struct Harness {
    state: State<u32>,
    handle: HandleView,
    adjacent: GapView,
    valid: GapView,
    foreign: GapView,
    calls: Signal<usize>,
    context: Signal<String>,
}
#[derive(Clone, Props)]
struct Props {
    output: Rc<RefCell<Option<Harness>>>,
}
impl PartialEq for Props {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.output, &other.output)
    }
}
fn root(props: Props) -> Element {
    let b = inputforge_app::ui::bindings().rail_sortable;
    let state = inputforge_gui_dx::components::sortable::use_sortable_state(b);
    let context = use_signal(|| "before".to_owned());
    let handle = (b.handle)(HandleProps {
        state,
        index: 0,
        group: 1,
        context: context.read().clone(),
        draggable: true,
    });
    let mut calls = use_signal(|| 0);
    let on_drop = Callback::new(move |_| {
        calls += 1;
    });
    let adjacent = (b.gap)(GapProps {
        state,
        gap_index: 1,
        group: 1,
        on_drop,
    });
    let valid = (b.gap)(GapProps {
        state,
        gap_index: 3,
        group: 1,
        on_drop,
    });
    let foreign = (b.gap)(GapProps {
        state,
        gap_index: 2,
        group: 2,
        on_drop,
    });
    let output = props.output;
    *output.borrow_mut() = Some(Harness {
        state,
        handle,
        adjacent,
        valid,
        foreign,
        calls,
        context,
    });
    rsx! {}
}
fn fixture() -> (VirtualDom, Harness) {
    let output = Rc::new(RefCell::new(None));
    let mut dom = VirtualDom::new_with_props(
        root,
        Props {
            output: Rc::clone(&output),
        },
    );
    dom.rebuild_in_place();
    let state = output.borrow().as_ref().unwrap().clone();
    (dom, state)
}
fn event() -> DragEvent {
    Event::new(Rc::new(DragData::from(serde_json::from_value::<SerializedDragData>(serde_json::json!({"mouse": SerializedMouseData::default(),"data_transfer":{"items":[],"files":[],"effect_allowed":"move","drop_effect":"move"}})).unwrap())),true)
}

#[test]
fn app_gap_clears_old_indicator_and_suppresses_adjacent_and_foreign_drops() {
    let (_dom, s) = fixture();
    s.handle.start.call(event());
    s.valid.over.call(event());
    assert_eq!(s.state.drop_target.peek().as_ref().unwrap().gap_index, 3);
    s.adjacent.over.call(event());
    assert!(s.state.drop_target.peek().is_none());
    assert!(!s.state.live_announcement.peek().is_empty());
    s.adjacent.drop.call(event());
    assert_eq!(*s.calls.peek(), 0);
    s.handle.start.call(event());
    s.foreign.over.call(event());
    assert!(s.state.drop_target.peek().as_ref().unwrap().invalid);
    s.foreign.drop.call(event());
    assert_eq!(*s.calls.peek(), 0);
    assert!(s.state.drag_from.peek().is_none());
}
#[test]
fn app_drop_delivers_once_then_clears_and_can_start_another_gesture() {
    let (_dom, s) = fixture();
    s.handle.start.call(event());
    s.valid.drop.call(event());
    s.valid.drop.call(event());
    assert_eq!(*s.calls.peek(), 1);
    assert!(s.state.drag_from.peek().is_none());
    s.handle.start.call(event());
    s.valid.drop.call(event());
    assert_eq!(*s.calls.peek(), 2);
}
#[test]
fn removed_owner_rejects_late_callbacks() {
    let (_dom, s) = fixture();
    s.handle.start.call(event());
    s.state.close.call(());
    s.handle.start.call(event());
    s.valid.drop.call(event());
    assert_eq!(*s.calls.peek(), 0);
    assert!(s.state.drag_from.peek().is_none());
}

#[test]
fn replacing_context_clears_active_source_before_an_obsolete_drop() {
    let (mut dom, mut s) = fixture();
    s.handle.start.call(event());
    s.context.set("replacement-profile-mode-tree".into());
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert!(s.state.drag_from.peek().is_none());
    s.valid.drop.call(event());
    assert_eq!(*s.calls.peek(), 0);
}
