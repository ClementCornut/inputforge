//! Live capture owns delivery and rejects stale consumers, epochs and native bindings.
#[path = "live_capture/fixture.rs"]
mod fixture;
use dioxus::prelude::*;
use fixture::{address, fixture, profile};
use inputforge_core::{action::Action, engine::EngineCommand};
use std::{sync::mpsc, time::Instant};

#[test]
fn sparse_native_input_is_delivered_once_after_debounce_with_the_engine_unlocked() {
    let (dom, f, received) = fixture();
    dom.in_scope(ScopeId::APP, || {
        let now = Instant::now();
        f.pending(now);
        f.tick(65, now);
        assert_eq!(f.results.borrow().len(), 0);
        f.tick(66, now);
        f.tick(200, now);
        assert_eq!(*f.results.borrow(), vec![address(1)]);
        assert!(!*f.active.peek());
        assert!(!*f.requests[0].listening.peek());
        assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    });
}

#[test]
fn stale_cancellation_does_not_cancel_the_new_capture_owner() {
    let (dom, f, _) = fixture();
    dom.in_scope(ScopeId::APP, || {
        f.start(0);
        f.start(1);
        f.bindings.cancel.call(f.requests[0].owner);
        assert!(*f.active.peek());
        assert!(!*f.requests[0].listening.peek());
        assert!(*f.requests[1].listening.peek());
        assert_eq!(*f.cancelled.borrow(), vec![0]);
        f.bindings.cancel.call(f.requests[1].owner);
        assert!(!*f.active.peek());
        assert_eq!(*f.cancelled.borrow(), vec![0, 1]);
    });
}

#[test]
fn rearming_the_same_consumer_preserves_its_workflow_and_uses_a_fresh_baseline() {
    let (dom, f, _) = fixture();
    dom.in_scope(ScopeId::APP, || {
        let now = Instant::now();
        f.pending(now);
        f.start(0);
        f.tick(80, now);
        f.tick(160, now);
        assert_eq!(f.results.borrow().len(), 0);
        assert!(
            f.cancelled.borrow().is_empty(),
            "recapture must not close its own workflow"
        );
        assert!(*f.requests[0].listening.peek());
    });
}

#[test]
fn delivery_waits_for_contention_and_completion_may_arm_another_capture() {
    let (dom, mut f, _) = fixture();
    dom.in_scope(ScopeId::APP, || {
        let next = f.clone();
        f.requests[0].on_capture = Callback::new(move |_| next.start(1));
        let now = Instant::now();
        f.pending(now);
        f.sample(66, now);
        let guard = f.state.write();
        f.controller.deliver();
        assert!(*f.requests[0].listening.peek());
        drop(guard);
        f.controller.deliver();
        assert!(!*f.requests[0].listening.peek());
        assert!(*f.requests[1].listening.peek());
        assert!(*f.active.peek());
    });
}

#[test]
fn pending_detection_rebaselines_after_generation_and_readiness_changes() {
    let (dom, f, _) = fixture();
    dom.in_scope(ScopeId::APP, || {
        let now = Instant::now();
        f.pending(now);
        f.state.write().session.generation += 1;
        f.tick(80, now);
        f.tick(160, now);
        assert_eq!(f.results.borrow().len(), 0);
        assert!(*f.active.peek());
        f.state.write().session.ready = false;
        f.button(1, false);
        f.tick(176, now);
        f.state.write().session.ready = true;
        f.tick(192, now);
        f.button(1, true);
        f.tick(208, now);
        f.tick(258, now);
        assert_eq!(*f.results.borrow(), vec![address(1)]);
    });
}

#[test]
fn delivered_candidate_is_revalidated_against_current_epoch_and_native_identity() {
    for mutation in 0..4 {
        let (dom, f, _) = fixture();
        dom.in_scope(ScopeId::APP, || {
            let now = Instant::now();
            f.pending(now);
            f.sample(66, now);
            match mutation {
                0 => f.state.write().session.generation += 1,
                1 => f.state.write().session.ready = false,
                2 => f.state.write().session.bindings[0].buttons.swap(0, 1),
                _ => f
                    .state
                    .write()
                    .input_cache
                    .evict_device(address(1).device().unwrap()),
            }
            f.controller.deliver();
            assert!(
                f.results.borrow().is_empty(),
                "stale mutation {mutation} must suppress delivery"
            );
            assert!(!*f.active.peek());
        });
    }
}

#[test]
fn changed_profile_mode_mapping_or_stage_cancels_pending_capture() {
    for mutation in 0..5 {
        let (dom, mut f, _) = fixture();
        dom.in_scope(ScopeId::APP, || {
            let now = Instant::now();
            f.pending(now);
            match mutation {
                0 => f.state.write().active_profile = Some(profile()),
                1 => f.requests[0].target.write().mode = "Other".into(),
                2 => f
                    .state
                    .write()
                    .active_profile
                    .as_mut()
                    .unwrap()
                    .set_mapping(&address(0), "Default", None, vec![Action::Invert]),
                3 => f.requests[0].target.write().field = "stage:1".into(),
                _ => f
                    .state
                    .write()
                    .active_profile
                    .as_mut()
                    .unwrap()
                    .set_mapping(&address(0), "Default", Some("Renamed".into()), vec![]),
            }
            f.tick(66, now);
            assert_eq!(f.results.borrow().len(), 0);
            assert!(!*f.active.peek(), "target mutation {mutation} must cancel");
        });
    }
}

#[test]
fn navigation_changes_are_revalidated_before_the_consumer_rerenders() {
    for change_mode in [false, true] {
        let (dom, mut f, _) = fixture();
        dom.in_scope(ScopeId::APP, || {
            let now = Instant::now();
            f.pending(now);
            f.sample(66, now);
            if change_mode {
                f.mode.set("Other".into());
            } else {
                f.selection.set(None);
            }
            f.controller.deliver();
            assert!(
                f.results.borrow().is_empty(),
                "stale rendered props must not deliver after navigation"
            );
            assert!(!*f.active.peek());
        });
    }
}

#[test]
fn offline_cancels_but_unready_and_running_do_not_lock_capture() {
    let (dom, f, _) = fixture();
    dom.in_scope(ScopeId::APP, || {
        let now = Instant::now();
        f.state.write().session.ready = false;
        f.state.write().engine_status = inputforge_core::state::EngineStatus::Running;
        f.start(0);
        f.tick(0, now);
        assert!(*f.active.peek());
        f.state.write().session.offline = true;
        f.tick(16, now);
        assert!(!*f.active.peek());
        assert_eq!(*f.cancelled.borrow(), vec![0]);
    });
}

#[test]
fn ambiguous_same_address_confirmation_precedes_delivery_and_failure_cancels() {
    for disconnected in [false, true] {
        let (dom, f, received) = fixture();
        dom.in_scope(ScopeId::APP, || {
            f.state.write().session.bindings[0].unavailable = vec![inputforge_core::types::InputId::Button { index: 0 }];
            let received = if disconnected { drop(received); None } else { Some(received) };
            let now = Instant::now();
            f.start(0);
            f.tick(0, now);
            f.button(0, true);
            f.tick(16, now);
            f.tick(66, now);
            if let Some(received) = received {
                assert!(matches!(received.try_recv().unwrap(), EngineCommand::ConfirmInputBinding { device, input }
                    if device == *address(0).device().unwrap() && input == *address(0).input_id().unwrap()));
                assert_eq!(*f.results.borrow(), vec![address(0)]);
                assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
            } else {
                assert_eq!(f.results.borrow().len(), 0);
                assert_eq!(*f.cancelled.borrow(), vec![0]);
            }
        });
    }
}
