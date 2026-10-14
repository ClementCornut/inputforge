//! App completion must not expose the old value between renderer frames.
use crate::keyboard_document as fake_document;
use dioxus::prelude::*;
use inputforge_core::{
    action::{Action, Mapping, OutputBehavior},
    mode::Modes,
    pipeline::ActionPathSegment,
    profile::Profile,
    state::AppState,
    types::{InputAddress, KeyCombo, PhysicalKey},
};
use inputforge_gui_dx::{
    capture,
    keyboard_capture::{Capture, Destination, Publication, Target},
};
use parking_lot::RwLock;
use std::{cell::RefCell, rc::Rc, sync::Arc};
type Controls = Rc<
    RefCell<
        Option<(
            Capture,
            Signal<Option<crate::snapshots::MappingKey>>,
            Signal<Destination>,
        )>,
    >,
>;
type Frames = Rc<RefCell<Vec<(bool, PhysicalKey)>>>;
fn harness() -> Element {
    let state = use_context::<Arc<RwLock<AppState>>>();
    let controller =
        use_hook(|| inputforge_app::keyboard_capture::Controller::new(Arc::clone(&state)));
    let bindings = controller.bindings();
    use_context_provider(|| bindings);
    let mode = use_signal(|| "Default".into());
    let selection = use_signal(|| Some(("Default".into(), InputAddress::Unbound)));
    let publication = Publication { mode, selection };
    use_context_provider(|| publication);
    use_hook(|| bindings.install.call(publication));
    use_drop(move || bindings.dispose.call(()));
    let profile = state.read().active_profile.clone().unwrap();
    let destination = use_signal(|| Destination::Edit(vec![ActionPathSegment::Index(0)]));
    let target = Target {
        mapping: capture::Target {
            profile: Some(profile.id().clone()),
            mode: "Default".into(),
            mapping: Some(InputAddress::Unbound),
            name: None,
            actions: Some(
                profile
                    .find_mapping(&InputAddress::Unbound, "Default")
                    .unwrap()
                    .actions
                    .clone(),
            ),
            field: "keyboard".into(),
        },
        destination: destination.read().clone(),
    };
    let mut key = use_signal(|| PhysicalKey::F11);
    let binding = super::use_keyboard_capture(
        target,
        use_callback(move |combo: KeyCombo| key.set(combo.key)),
    );
    *use_context::<Controls>().borrow_mut() = Some((binding, selection, destination));
    use_context::<Frames>()
        .borrow_mut()
        .push((*binding.active.read(), *key.read()));
    rsx! { div {} }
}
#[test]
fn capture_stays_active_until_new_key_is_delivered() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (mut dom, controls, frames, document) = fixture();
            let binding = controls.borrow().unwrap().0;
            dom.in_scope(ScopeId::APP, || binding.start.call(()));
            dom.render_immediate_to_vec();
            assert_eq!(frames.borrow().last(), Some(&(true, PhysicalKey::F11)));
            frames.borrow_mut().clear();
            document.key(0, "keydown", "F12", "F12");
            for _ in 0..4 {
                tokio::task::yield_now().await;
                dom.render_immediate_to_vec();
            }
            assert!(
                !frames.borrow().contains(&(false, PhysicalKey::F11)),
                "idle must never expose the old key: {:?}",
                frames.borrow()
            );
            assert_eq!(frames.borrow().last(), Some(&(false, PhysicalKey::F12)));
            drop(dom);
            assert_eq!(document.shutdowns(), 1);
            assert_eq!(document.cleanups(), 1);
        });
}

fn fixture() -> (VirtualDom, Controls, Frames, Rc<fake_document::Document>) {
    let controls = Controls::default();
    let frames = Frames::default();
    let document = Rc::new(fake_document::Document::default());
    let profile = Profile::new(
        "Display".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![Mapping {
            input: InputAddress::Unbound,
            mode: "Default".into(),
            name: None,
            actions: vec![Action::MapToKeyboard {
                key: KeyCombo {
                    key: PhysicalKey::F11,
                    modifiers: vec![],
                },
                behavior: OutputBehavior::Hold,
            }],
        }],
        vec![],
        "Default".into(),
    );
    let mut dom = VirtualDom::new(harness);
    dom.provide_root_context(Arc::new(RwLock::new(AppState::with_profile(profile))));
    dom.provide_root_context(Rc::clone(&controls));
    dom.provide_root_context(Rc::clone(&frames));
    dom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    (dom, controls, frames, document)
}

#[test]
fn target_or_navigation_change_disarms_capture_without_swallowing_another_key() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            for navigation in [false, true] {
                let (mut dom, controls, _, _) = fixture();
                let (capture, mut selection, mut destination) = controls.borrow().unwrap();
                dom.in_scope(ScopeId::APP, || capture.start.call(()));
                dom.render_immediate_to_vec();
                assert!(*capture.active.peek());
                dom.in_scope(ScopeId::APP, || {
                    if navigation {
                        selection.set(None);
                    } else {
                        destination.set(Destination::Edit(vec![ActionPathSegment::Index(9)]));
                    }
                });
                for _ in 0..4 {
                    tokio::task::yield_now().await;
                    dom.render_immediate_to_vec();
                }
                assert!(
                    !*capture.active.peek(),
                    "obsolete capture must disarm before another key arrives"
                );
            }
        });
}
