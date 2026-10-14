//! Keyboard capture rejects stale destinations and keeps pending completion active.
#[path = "keyboard_capture/document.rs"]
mod document;
#[path = "keyboard_capture/fixture.rs"]
mod fixture;
use dioxus::prelude::*;
use fixture::{drive, fixture, profile, runtime};
use inputforge_core::{action::Action, types::PhysicalKey};

#[test]
fn pending_capture_delivers_once_with_active_display_and_engine_unlocked() {
    runtime().block_on(async {
        let (mut dom, f, document) = fixture();
        dom.in_scope(ScopeId::APP, || f.start(0));
        document.key(0, "keydown", "a", "KeyQ");
        document.key(0, "keydown", "a", "KeyQ");
        drive(&mut dom).await;
        assert!(*f.requests[0].active.peek());
        assert!(*f.requests[0].pending.peek());
        assert_eq!(*f.results.borrow(), vec![]);
        dom.in_scope(ScopeId::APP, || {
            f.bindings.deliver.call(f.requests[0].owner);
            f.bindings.deliver.call(f.requests[0].owner);
        });
        assert_eq!(*f.results.borrow(), vec![(0, PhysicalKey::KeyQ, true)]);
        assert!(!*f.requests[0].active.peek());
        assert!(!*f.requests[0].pending.peek());
    });
}

#[test]
fn replacement_and_rearm_ignore_stale_cancel_and_queued_events() {
    runtime().block_on(async {
        let (mut dom, f, document) = fixture();
        dom.in_scope(ScopeId::APP, || f.start(0));
        document.key(0, "keydown", "Escape", "Escape");
        dom.in_scope(ScopeId::APP, || {
            f.start(1);
            f.bindings.cancel.call(f.requests[0].owner);
        });
        drive(&mut dom).await;
        assert!(!*f.requests[0].active.peek());
        assert!(*f.requests[1].active.peek());
        dom.in_scope(ScopeId::APP, || f.start(1));
        document.key(1, "keydown", "J", "KeyJ");
        document.key(2, "keydown", "K", "KeyK");
        drive(&mut dom).await;
        dom.in_scope(ScopeId::APP, || {
            f.bindings.deliver.call(f.requests[1].owner);
        });
        assert_eq!(*f.results.borrow(), vec![(1, PhysicalKey::KeyK, true)]);
    });
}

#[test]
fn pending_completion_is_discarded_on_cancel_or_replacement() {
    runtime().block_on(async {
        for replace in [false, true] {
            let (mut dom, f, document) = fixture();
            dom.in_scope(ScopeId::APP, || f.start(0));
            document.key(0, "keydown", "J", "KeyJ");
            drive(&mut dom).await;
            dom.in_scope(ScopeId::APP, || {
                if replace {
                    f.start(1);
                } else {
                    f.bindings.cancel.call(f.requests[0].owner);
                }
                f.bindings.deliver.call(f.requests[0].owner);
            });
            assert_eq!(*f.results.borrow(), vec![]);
            assert!(!*f.requests[0].active.peek());
        }
    });
}

#[test]
fn current_profile_mapping_and_rendered_destination_are_revalidated() {
    runtime().block_on(async {
        for mutation in 0..7 {
            let (mut dom, mut f, document) = fixture();
            dom.in_scope(ScopeId::APP, || f.start(0));
            document.key(0, "keydown", "J", "KeyJ");
            drive(&mut dom).await;
            dom.in_scope(ScopeId::APP, || {
                match mutation {
                    0 => f.state.write().active_profile = Some(profile()),
                    1 => f.mode.set("Other".into()),
                    2 => f.selection.set(None),
                    3 => {
                        f.state
                            .write()
                            .active_profile
                            .as_mut()
                            .unwrap()
                            .set_mapping(
                                &inputforge_core::types::InputAddress::Unbound,
                                "Default",
                                None,
                                vec![Action::Invert],
                            );
                    }
                    4 => {
                        let actions = f.requests[0].target.peek().mapping.actions.clone().unwrap();
                        f.state
                            .write()
                            .active_profile
                            .as_mut()
                            .unwrap()
                            .set_mapping(
                                &inputforge_core::types::InputAddress::Unbound,
                                "Default",
                                Some("Renamed".into()),
                                actions,
                            );
                    }
                    5 => f.requests[0].target.write().mapping.field = "different field".into(),
                    _ => {
                        f.requests[0].target.write().destination =
                            inputforge_gui_dx::keyboard_capture::Destination::Edit(vec![]);
                    }
                }
                f.bindings.deliver.call(f.requests[0].owner);
            });
            assert_eq!(
                *f.results.borrow(),
                vec![],
                "mutation {mutation} must reject delivery"
            );
            assert!(!*f.requests[0].active.peek());
        }
    });
}

#[test]
fn completion_can_rearm_the_same_owner_without_clearing_its_new_state() {
    runtime().block_on(async {
        let (mut dom, mut f, document) = fixture();
        dom.in_scope(ScopeId::APP, || {
            let next = f.clone();
            f.requests[0].on_commit = Callback::new(move |_| next.start(0));
            f.start(0);
        });
        document.key(0, "keydown", "J", "KeyJ");
        drive(&mut dom).await;
        dom.in_scope(ScopeId::APP, || {
            f.bindings.deliver.call(f.requests[0].owner);
        });
        assert!(*f.requests[0].active.peek());
        assert!(!*f.requests[0].pending.peek());
        assert_eq!(document.listeners.borrow().len(), 1);
    });
}

#[test]
fn escape_unsupported_keys_and_scope_drop_release_the_listener() {
    runtime().block_on(async {
        for (key, code, hint) in [
            ("Escape", "Escape", None),
            ("AudioVolumeUp", "AudioVolumeUp", Some("Unsupported key")),
        ] {
            let (mut dom, f, document) = fixture();
            dom.in_scope(ScopeId::APP, || f.start(0));
            document.key(0, "keydown", key, code);
            drive(&mut dom).await;
            assert!(!*f.requests[0].active.peek());
            assert_eq!(*f.requests[0].hint.peek(), hint);
            assert_eq!(*f.results.borrow(), vec![]);
            dom.in_scope(ScopeId::APP, || f.start(0));
            drop(dom);
            assert_eq!(document.shutdowns(), 1);
            assert_eq!(document.cleanups(), 1);
        }
    });
}

#[test]
fn root_listener_is_installed_once_across_rearms_and_removed_on_drop() {
    let (dom, f, document) = fixture();
    assert_eq!(
        document.listeners.borrow().len(),
        1,
        "root must install the passive listener before editor shortcuts"
    );
    dom.in_scope(ScopeId::APP, || {
        f.start(0);
        f.start(1);
        f.start(1);
    });
    assert_eq!(
        document.listeners.borrow().len(),
        1,
        "rearming must reuse the root listener"
    );
    drop(dom);
    assert_eq!(document.shutdowns(), 1);
    assert_eq!(document.cleanups(), 1);
}

#[test]
fn old_root_shutdown_cannot_dispose_a_reinstalled_listener() {
    runtime().block_on(async {
        let (mut dom, f, document) = fixture();
        dom.in_scope(ScopeId::APP, || {
            f.start(0);
            f.bindings
                .install
                .call(inputforge_gui_dx::keyboard_capture::Publication {
                    mode: f.mode,
                    selection: f.selection,
                });
            f.start(0);
        });
        drive(&mut dom).await;
        assert!(
            *f.requests[0].active.peek(),
            "old shutdown must leave the current listener and request intact"
        );
        document.key_on(1, 2, "keydown", "J", "KeyJ");
        drive(&mut dom).await;
        dom.in_scope(ScopeId::APP, || {
            f.bindings.deliver.call(f.requests[0].owner);
        });
        assert_eq!(*f.results.borrow(), vec![(0, PhysicalKey::KeyJ, true)]);
        drop(dom);
        assert_eq!(document.shutdowns(), 2);
        assert_eq!(document.cleanups(), 2);
    });
}

#[test]
fn another_modes_name_and_running_unready_routing_do_not_reject_keyboard_capture() {
    runtime().block_on(async {
        let (mut dom, f, document) = fixture();
        dom.in_scope(ScopeId::APP, || {
            let actions = f.requests[0].target.peek().mapping.actions.clone().unwrap();
            let mut state = f.state.write();
            state.active_profile.as_mut().unwrap().set_mapping(
                &inputforge_core::types::InputAddress::Unbound,
                "Other",
                Some("Other name".into()),
                actions,
            );
            state.engine_status = inputforge_core::state::EngineStatus::Running;
            state.session.ready = false;
            drop(state);
            f.start(0);
        });
        document.key(0, "keydown", "J", "KeyJ");
        drive(&mut dom).await;
        dom.in_scope(ScopeId::APP, || {
            f.bindings.deliver.call(f.requests[0].owner);
        });
        assert_eq!(*f.results.borrow(), vec![(0, PhysicalKey::KeyJ, true)]);
    });
}

#[test]
fn removed_mapping_and_invalid_destinations_never_deliver() {
    use inputforge_core::pipeline::ActionPathSegment::{Branch, Index};
    use inputforge_gui_dx::keyboard_capture::Destination::{Edit, Insert};
    runtime().block_on(async {
        for destination in [
            Edit(vec![]),
            Edit(vec![Index(9)]),
            Edit(vec![
                Index(0),
                Branch(inputforge_core::action::ActionBranch::ConditionalTrue),
                Index(0),
            ]),
            Insert(vec![Index(2)]),
            Insert(vec![Branch(
                inputforge_core::action::ActionBranch::ConditionalTrue,
            )]),
        ] {
            let (dom, mut f, _) = fixture();
            dom.in_scope(ScopeId::APP, || {
                f.requests[0].target.write().destination = destination;
                f.bindings.start.call(f.requests[0]);
                assert!(!*f.requests[0].active.peek());
                assert!(!*f.requests[0].pending.peek());
            });
        }
        let (mut dom, f, document) = fixture();
        dom.in_scope(ScopeId::APP, || f.start(0));
        document.key(0, "keydown", "J", "KeyJ");
        drive(&mut dom).await;
        dom.in_scope(ScopeId::APP, || {
            f.state
                .write()
                .active_profile
                .as_mut()
                .unwrap()
                .remove_mapping(&inputforge_core::types::InputAddress::Unbound, "Default");
            f.bindings.deliver.call(f.requests[0].owner);
        });
        assert_eq!(*f.results.borrow(), vec![]);
        assert!(!*f.requests[0].active.peek());
    });
}

#[test]
fn a_new_owner_clears_the_previous_owners_cancel_hint() {
    runtime().block_on(async {
        let (mut dom, f, document) = fixture();
        dom.in_scope(ScopeId::APP, || f.start(0));
        document.key(0, "keydown", "AudioVolumeUp", "AudioVolumeUp");
        drive(&mut dom).await;
        assert_eq!(*f.requests[0].hint.peek(), Some("Unsupported key"));
        dom.in_scope(ScopeId::APP, || f.start(1));
        assert_eq!(
            *f.requests[0].hint.peek(),
            None,
            "hints must follow the current capture owner"
        );
        assert_eq!(*f.requests[1].hint.peek(), None);
    });
}

#[test]
fn valid_nested_keyboard_fields_and_branch_insertions_receive_completion() {
    use inputforge_core::{
        action::{ActionBranch, Condition},
        pipeline::ActionPathSegment::{Branch, Index},
    };
    use inputforge_gui_dx::keyboard_capture::Destination::{Edit, Insert};
    runtime().block_on(async {
        for destination in [
            Edit(vec![
                Index(0),
                Branch(ActionBranch::ConditionalTrue),
                Index(0),
            ]),
            Insert(vec![
                Index(0),
                Branch(ActionBranch::ConditionalTrue),
                Index(1),
            ]),
        ] {
            let (mut dom, mut f, document) = fixture();
            dom.in_scope(ScopeId::APP, || {
                let key_action = f.requests[0].target.peek().mapping.actions.clone().unwrap();
                let actions = vec![Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: inputforge_core::types::InputAddress::Unbound,
                    },
                    if_true: key_action,
                    if_false: vec![],
                }];
                f.state
                    .write()
                    .active_profile
                    .as_mut()
                    .unwrap()
                    .set_mapping(
                        &inputforge_core::types::InputAddress::Unbound,
                        "Default",
                        None,
                        actions.clone(),
                    );
                f.requests[0].target.write().mapping.actions = Some(actions);
                f.requests[0].target.write().destination = destination;
                f.start(0);
            });
            document.key(0, "keydown", "J", "KeyJ");
            drive(&mut dom).await;
            dom.in_scope(ScopeId::APP, || {
                f.bindings.deliver.call(f.requests[0].owner);
            });
            assert_eq!(*f.results.borrow(), vec![(0, PhysicalKey::KeyJ, true)]);
        }
    });
}
