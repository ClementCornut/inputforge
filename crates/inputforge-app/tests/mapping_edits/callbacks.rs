use super::support::*;
use dioxus::prelude::*;
use inputforge_app::mapping_edits::Controller;
use inputforge_core::action::Action;
use inputforge_gui_dx::mapping_edits::{Bindings, Group, Publication, View};
use std::{cell::RefCell, rc::Rc};
struct Fixture {
    controller: Controller,
    publication: RefCell<Option<Publication>>,
    bindings: RefCell<Option<Bindings>>,
}
fn root() -> Element {
    let fixture = use_context::<Rc<Fixture>>();
    let history = use_signal(View::default);
    let mode = use_signal(|| "Default".to_owned());
    let selection = use_signal(|| Some(key()));
    let bindings = use_hook(|| fixture.controller.bindings());
    use_hook(|| {
        let publication = Publication {
            history,
            mode,
            selection,
        };
        *fixture.publication.borrow_mut() = Some(publication);
        *fixture.bindings.borrow_mut() = Some(bindings);
        bindings.install.call(publication);
    });
    use_drop(move || bindings.dispose.call(()));
    rsx! { div { "{history.read().stacks.len()}" } }
}
fn mount(controller: Controller) -> (VirtualDom, Rc<Fixture>) {
    let fixture = Rc::new(Fixture {
        controller,
        publication: RefCell::default(),
        bindings: RefCell::default(),
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&fixture));
    dom.rebuild_in_place();
    (dom, fixture)
}
#[test]
fn callbacks_revalidate_navigation_publish_labels_and_invalidate_external_history() {
    let (controller, state, rx) = fixture(vec![Action::Invert]);
    let (mut dom, f) = mount(controller);
    let bindings = f.bindings.borrow().unwrap();
    let mut p = f.publication.borrow().unwrap();
    let before = mapping(vec![Action::Invert]);
    p.selection.set(None);
    assert!(
        !bindings
            .dispatch
            .call(replace(&state, before.clone(), 0, value(0.5)))
    );
    p.selection.set(Some(key()));
    p.mode.set("Other".into());
    assert!(
        !bindings
            .dispatch
            .call(replace(&state, before.clone(), 0, value(0.5)))
    );
    p.mode.set("Default".into());
    assert!(
        bindings
            .dispatch
            .call(replace(&state, before, 0, value(0.5)))
    );
    assert_eq!(p.history.peek().stacks[&key()].undo.len(), 1);
    apply(&state, &rx);
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, &key().0);
    bindings.reconcile.call(());
    assert!(p.history.peek().stacks.is_empty());
    dom.render_immediate_to_vec();
    drop(dom);
    assert!(f.controller.view().stacks.is_empty());
}
#[test]
fn navigation_round_trip_splits_a_nudge_group() {
    for contended in [false, true] {
        let (controller, state, rx) = fixture(vec![Action::Invert]);
        let (_dom, f) = mount(controller);
        let bindings = f.bindings.borrow().unwrap();
        let mut p = f.publication.borrow().unwrap();
        let start = now();
        let mut r = replace(&state, mapping(vec![Action::Invert]), 0, value(0.25));
        r.group = Some(Group {
            path: stage(0),
            key: "right".into(),
        });
        assert!(f.controller.dispatch(r, start));
        let before = apply(&state, &rx);
        let contention = contended.then(|| state.write());
        p.selection.set(None);
        bindings.reconcile.call(());
        p.selection.set(Some(key()));
        bindings.reconcile.call(());
        drop(contention);
        let mut r = replace(&state, before, 0, value(0.5));
        r.group = Some(Group {
            path: stage(0),
            key: "right".into(),
        });
        assert!(
            f.controller
                .dispatch(r, start + std::time::Duration::from_millis(100))
        );
        assert_eq!(p.history.peek().stacks[&key()].undo.len(), 2);
    }
}

#[test]
fn disposing_and_remounting_a_controller_drops_history_and_keeps_queue_serials_monotonic() {
    let (controller, state, rx) = fixture(vec![Action::Invert]);
    let (dom, f) = mount(controller.clone());
    assert!(f.bindings.borrow().unwrap().dispatch.call(replace(
        &state,
        mapping(vec![Action::Invert]),
        0,
        value(0.25)
    )));
    let before = apply(&state, &rx);
    let serial = state.read().mapping_edit_barrier.unwrap();
    drop(dom);
    let (_dom, f) = mount(controller);
    assert!(
        f.publication
            .borrow()
            .unwrap()
            .history
            .peek()
            .stacks
            .is_empty()
    );
    assert!(
        f.bindings
            .borrow()
            .unwrap()
            .dispatch
            .call(replace(&state, before, 0, value(0.5)))
    );
    apply(&state, &rx);
    let next = state.read().mapping_edit_barrier.unwrap();
    assert_eq!(next.0, serial.0);
    assert!(next.1 > serial.1);
    assert_eq!(
        f.publication.borrow().unwrap().history.peek().stacks[&key()]
            .undo
            .len(),
        1
    );
}
#[test]
fn rail_rename_uses_current_actions_without_requiring_editor_selection() {
    let (controller, state, rx) = fixture(vec![Action::Invert]);
    let (_dom, f) = mount(controller);
    let mut p = f.publication.borrow().unwrap();
    p.selection.set(None);
    let bindings = f.bindings.borrow().unwrap();
    let mut r = request(
        &state,
        mapping(vec![]),
        inputforge_gui_dx::mapping_edits::Edit::Rename("Rail".into()),
    );
    r.editor = false;
    assert!(bindings.dispatch.call(r.clone()));
    let renamed = apply(&state, &rx);
    assert_eq!(renamed.actions, vec![Action::Invert]);
    assert_eq!(renamed.name.as_deref(), Some("Rail"));
    r.edit = inputforge_gui_dx::mapping_edits::Edit::Rename("Stale".into());
    assert!(!bindings.dispatch.call(r));
    p.selection.set(Some(key()));
    assert!(bindings.undo.call(key()));
    assert_eq!(apply(&state, &rx).name, None);
}
#[test]
fn forget_callback_invalidates_only_the_overwritten_mapping_history() {
    let (controller, state, rx) = fixture(vec![Action::Invert]);
    let (_dom, f) = mount(controller);
    let bindings = f.bindings.borrow().unwrap();
    let mut p = f.publication.borrow().unwrap();
    assert!(bindings.dispatch.call(replace(
        &state,
        mapping(vec![Action::Invert]),
        0,
        value(0.25)
    )));
    apply(&state, &rx);
    let mut other = mapping(vec![Action::Invert]);
    other.mode = "Other".into();
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &other.input,
        &other.mode,
        None,
        other.actions.clone(),
    );
    let other_key = (other.mode.clone(), other.input.clone());
    p.mode.set("Other".into());
    p.selection.set(Some(other_key.clone()));
    assert!(
        bindings
            .dispatch
            .call(replace(&state, other, 0, value(0.5)))
    );
    apply(&state, &rx);
    bindings.forget.call(key());
    assert_eq!(p.history.peek().stacks.len(), 1);
    assert_eq!(p.history.peek().stacks[&other_key].undo.len(), 1);
    p.mode.set("Default".into());
    p.selection.set(Some(key()));
    assert!(!bindings.undo.call(key()));
}
