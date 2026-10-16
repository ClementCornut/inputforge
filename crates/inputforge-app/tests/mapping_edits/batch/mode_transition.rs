use super::support::*;
use inputforge_gui_dx::{
    batch_mapping::Intent,
    mode_management::{Operation, Request, Scope},
};
#[test]
fn pending_mode_mutation_retains_batch_without_delivery_or_completion() {
    let (_dom, f, rx) = mount();
    let scope = Scope {
        profile: f
            .state
            .read()
            .active_profile
            .as_ref()
            .map(|p| p.id().clone()),
        editing: "Default".into(),
        revision: f.state.read().mode_revision,
    };
    f.c.mode_action(Request {
        scope,
        operation: Operation::Rename {
            from: "Other".into(),
            raw: "Landing".into(),
            blur: false,
        },
    })
    .unwrap();
    f.act(Intent::Apply);
    assert!(f.model().error.is_some());
    assert_eq!(f.completed.get(), 0);
    rx.try_recv().unwrap();
    rx.try_recv().unwrap();
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
