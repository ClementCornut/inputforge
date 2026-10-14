use super::*;
#[test]
fn replaced_mount_rejects_old_callbacks_and_old_disposal() {
    let (_dom, f, rx) = mount();
    let old = f.request(Intent::Apply);
    let view = f.view.borrow().unwrap();
    let mode = f.mode.borrow().unwrap();
    let b = f.batch.borrow().unwrap();
    let applied = f.applied.borrow().unwrap();
    let owner = b
        .install
        .call(inputforge_gui_dx::batch_mapping::Publication {
            view,
            mode,
            applied,
        });
    assert_ne!(owner, old.owner);
    b.dispose.call(old.owner);
    b.dispatch.call(old);
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    f.act(Intent::Apply);
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::SetMappingsBulk { .. }
    ));
}
#[test]
fn disposed_root_and_wizard_reject_saved_callbacks() {
    for root in [false, true] {
        let (_dom, f, rx) = mount();
        let request = f.request(Intent::Apply);
        if root {
            f.mapping.borrow().unwrap().dispose.call(());
        } else {
            f.batch.borrow().unwrap().dispose.call(request.owner);
        }
        f.batch.borrow().unwrap().dispatch.call(request);
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
        assert_eq!(f.completed.get(), 0);
    }
}

#[test]
fn completion_is_reentrant_and_runs_after_engine_and_controller_guards_are_released() {
    use dioxus::prelude::*;
    let (dom, f, rx) = mount();
    let callback = std::rc::Rc::clone(&f);
    let applied = dom.in_scope(ScopeId::APP, || {
        Callback::new(move |count| {
            assert!(callback.state.try_write().is_some());
            callback.c.end_group();
            callback.completed.set(count);
        })
    });
    let bindings = f.batch.borrow().unwrap();
    bindings
        .install
        .call(inputforge_gui_dx::batch_mapping::Publication {
            view: f.view.borrow().unwrap(),
            mode: f.mode.borrow().unwrap(),
            applied,
        });
    f.act(Intent::Apply);
    assert_eq!(f.completed.get(), 5);
    assert_eq!(commands(&f.state, &rx).len(), 5);
}
