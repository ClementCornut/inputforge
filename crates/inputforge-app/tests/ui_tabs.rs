//! App tab policy uses observations without DOM references or engine handles.
use dioxus::prelude::*;
use inputforge_gui_dx::{
    components::TabsOrientation,
    controls::{TabState, TabsProps, TabsView},
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Copy)]
struct State {
    view: TabsView,
    value: Signal<String>,
    disabled: Signal<bool>,
    accept: Signal<bool>,
}
#[derive(Clone, Props)]
struct Props {
    output: Rc<RefCell<Option<State>>>,
}
impl PartialEq for Props {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.output, &other.output)
    }
}
fn root(props: Props) -> Element {
    let mut value = use_signal(|| "a".to_owned());
    let disabled = use_signal(|| false);
    let accept = use_signal(|| false);
    let bindings = inputforge_app::ui::bindings();
    let view = (bindings.tabs)(TabsProps {
        value: value.into(),
        disabled: disabled.into(),
        onchange: Callback::new(move |id| {
            if accept() {
                value.set(id);
            }
        }),
    });
    let output = props.output;
    *output.borrow_mut() = Some(State {
        view,
        value,
        disabled,
        accept,
    });
    rsx! {}
}
fn fixture() -> (VirtualDom, State) {
    let output = Rc::new(RefCell::new(None));
    let mut dom = VirtualDom::new_with_props(
        root,
        Props {
            output: Rc::clone(&output),
        },
    );
    dom.rebuild_in_place();
    let state = output.borrow().unwrap();
    (dom, state)
}
fn entries() -> Vec<TabState> {
    [("a", false), ("b", true), ("c", false)]
        .into_iter()
        .map(|(id, disabled)| TabState {
            id: id.into(),
            disabled,
        })
        .collect()
}
fn key(k: Key) -> KeyboardEvent {
    Event::new(
        Rc::new(KeyboardData::from(SerializedKeyboardData::new(
            k,
            Code::Unidentified,
            Location::Standard,
            false,
            Modifiers::empty(),
            false,
        ))),
        true,
    )
}
#[test]
fn automatic_focus_waits_for_acceptance_and_drops_removed_targets() {
    let (_dom, mut s) = fixture();
    s.view
        .key
        .call((key(Key::ArrowRight), entries(), TabsOrientation::Horizontal));
    assert_eq!(*s.value.peek(), "a");
    assert!(s.view.focus.call((None, entries())).target.is_none());
    s.value.set("c".into());
    assert_eq!(
        s.view.focus.call((None, entries())).target.as_deref(),
        Some("c")
    );
    assert!(s.view.focus.call((None, entries())).target.is_none());
    s.view
        .key
        .call((key(Key::ArrowLeft), entries(), TabsOrientation::Horizontal));
    let c_only = vec![entries().pop().unwrap()];
    assert!(s.view.focus.call((None, c_only)).target.is_none());
    assert!(s.view.pending.peek().is_none());
}
#[test]
fn explicit_requests_repeat_and_disabled_targets_or_clusters_do_not_focus() {
    let (_dom, mut s) = fixture();
    for _ in 0..2 {
        let result = s.view.focus.call((Some("a".into()), entries()));
        assert_eq!(result.target.as_deref(), Some("a"));
        assert!(result.consumed);
    }
    let disabled = s.view.focus.call((Some("b".into()), entries()));
    assert!(disabled.consumed && disabled.target.is_none());
    s.disabled.set(true);
    let disabled = s.view.focus.call((Some("a".into()), entries()));
    assert!(disabled.consumed && disabled.target.is_none());
    s.view.click.call(("c".into(), false));
    assert_eq!(*s.value.peek(), "a");
}
#[test]
fn vertical_keys_and_all_disabled_or_empty_lists_are_bounded() {
    let (_dom, mut s) = fixture();
    s.accept.set(true);
    s.view
        .key
        .call((key(Key::ArrowRight), entries(), TabsOrientation::Vertical));
    assert_eq!(*s.value.peek(), "a");
    s.view
        .key
        .call((key(Key::ArrowDown), entries(), TabsOrientation::Vertical));
    assert_eq!(*s.value.peek(), "c");
    s.view
        .key
        .call((key(Key::ArrowDown), entries(), TabsOrientation::Vertical));
    assert_eq!(*s.value.peek(), "a");
    let mut disabled = entries();
    for entry in &mut disabled {
        entry.disabled = true;
    }
    s.view
        .key
        .call((key(Key::End), disabled, TabsOrientation::Horizontal));
    s.view
        .key
        .call((key(Key::Home), vec![], TabsOrientation::Horizontal));
    assert_eq!(*s.value.peek(), "a");
}
#[test]
fn explicit_focus_consumes_pending_keyboard_focus_in_the_same_turn() {
    let (_dom, mut s) = fixture();
    s.accept.set(true);
    s.view
        .key
        .call((key(Key::ArrowRight), entries(), TabsOrientation::Horizontal));
    assert_eq!(
        s.view
            .focus
            .call((Some("c".into()), entries()))
            .target
            .as_deref(),
        Some("c")
    );
    assert!(
        s.view.focus.call((None, entries())).target.is_none(),
        "one request must not focus twice"
    );
}
