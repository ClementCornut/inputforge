//! Capture completion must not expose the old value between renders.

use super::{
    CaptureOutcome, KeyboardCaptureBinding, KeyboardCaptureContext, KeyboardCaptureUpdate,
    apply_update, use_keyboard_capture, use_keyboard_capture_provider,
};
use dioxus::core::NoOpMutations;
use dioxus::prelude::*;
use inputforge_core::types::{KeyCombo, PhysicalKey};
use std::cell::RefCell;
use std::rc::Rc;

type Controls = Rc<RefCell<Option<(KeyboardCaptureContext, KeyboardCaptureBinding)>>>;
type Frames = Rc<RefCell<Vec<(bool, PhysicalKey)>>>;

fn harness() -> Element {
    let context = use_keyboard_capture_provider();
    let mut key = use_signal(|| PhysicalKey::F11);
    let binding = use_keyboard_capture(use_callback(move |combo: KeyCombo| key.set(combo.key)));
    *use_context::<Controls>().borrow_mut() = Some((context, binding));
    use_context::<Frames>()
        .borrow_mut()
        .push((*binding.active.read(), *key.read()));
    rsx! { div {} }
}

#[test]
fn capture_stays_active_until_new_key_is_delivered() {
    let controls = Controls::default();
    let frames = Frames::default();
    let mut dom = VirtualDom::new(harness);
    dom.provide_root_context(Rc::clone(&controls));
    dom.provide_root_context(Rc::clone(&frames));
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);
    let (capture, binding) = controls.borrow().expect("capture must be mounted");
    dom.runtime()
        .in_scope(ScopeId::APP, || binding.start.call(()));
    dom.render_immediate(&mut NoOpMutations);
    assert_eq!(frames.borrow().last(), Some(&(true, PhysicalKey::F11)));
    frames.borrow_mut().clear();
    let owner = capture.active_owner.peek().expect("capture must be active");
    dom.runtime().in_scope(ScopeId::APP, || {
        apply_update(
            KeyboardCaptureUpdate {
                owner,
                outcome: CaptureOutcome::Commit(KeyCombo {
                    key: PhysicalKey::F12,
                    modifiers: Vec::new(),
                }),
            },
            capture.active_owner,
            capture.hint_owner,
            capture.hint,
            capture.delivery,
        );
    });
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);
    assert!(
        !frames.borrow().contains(&(false, PhysicalKey::F11)),
        "idle must never expose the old key: {:?}",
        frames.borrow()
    );
    assert_eq!(frames.borrow().last(), Some(&(false, PhysicalKey::F12)));
}
