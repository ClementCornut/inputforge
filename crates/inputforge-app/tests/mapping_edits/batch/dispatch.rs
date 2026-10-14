use super::*;
#[test]
fn failed_delivery_keeps_draft_history_and_never_completes() {
    let (_dom, f, rx) = mount();
    f.act(Intent::Exclude(RowKind::Hat));
    let before = f.model().rows;
    drop(rx);
    f.act(Intent::Apply);
    assert_eq!(f.model().rows, before);
    assert!(f.model().error.is_some());
    assert_eq!(f.completed.get(), 0);
    assert_eq!(
        f.state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .len(),
        1
    );
}
#[test]
fn empty_batch_dispatches_no_snapshot_or_completion() {
    let (_dom, f, rx) = mount();
    for kind in [RowKind::Axis, RowKind::Button, RowKind::Hat] {
        f.act(Intent::Exclude(kind));
    }
    f.act(Intent::Apply);
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(f.completed.get(), 0);
}
#[test]
fn late_collisions_are_skipped_and_queued_batch_is_visible_to_the_next_apply() {
    let (_dom, f, rx) = mount();
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&address(0), "Default", None, vec![Action::Invert]);
    f.act(Intent::Apply);
    assert_eq!(f.completed.get(), 4);
    f.act(Intent::Apply);
    let entries = commands(&f.state, &rx);
    assert_eq!(entries.len(), 4);
    assert!(!entries.iter().any(|e| e.input == address(0)));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    f.refresh();
    assert_eq!(f.model().counts.skip, 5);
}
#[test]
fn batch_invalidates_affected_history_and_preserves_unrelated_mode() {
    let (_dom, f, rx) = mount();
    let r = super::super::support::replace(
        &f.state,
        super::super::support::mapping(vec![Action::Invert]),
        0,
        super::super::support::value(0.25),
    );
    assert!(f.c.dispatch(r, std::time::Instant::now()));
    super::super::support::apply(&f.state, &rx);
    let before = inputforge_core::action::Mapping {
        input: address(0),
        mode: "Default".into(),
        name: None,
        actions: vec![Action::Invert],
    };
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&before.input, "Default", None, before.actions.clone());
    assert!(f.c.dispatch(
        super::super::support::replace(&f.state, before, 0, super::super::support::value(0.5)),
        std::time::Instant::now()
    ));
    super::super::support::apply(&f.state, &rx);
    f.act(Intent::Replace(RowKind::Axis, 0));
    f.act(Intent::Apply);
    commands(&f.state, &rx);
    assert!(
        !f.c.view()
            .stacks
            .contains_key(&("Default".into(), address(0)))
    );
    assert!(
        f.c.view()
            .stacks
            .contains_key(&super::super::support::key())
    );
}

#[test]
fn alias_snapshot_label_and_failed_prevalidation_do_not_claim_execution() {
    let (_dom, f, rx) = mount();
    f.state.write().device_aliases.insert(
        inputforge_core::types::DeviceId("batch".into()),
        "My stick".into(),
    );
    f.act(Intent::Apply);
    let EngineCommand::SetMappingsBulk {
        snapshot_label,
        entries,
    } = rx.try_recv().unwrap()
    else {
        panic!("batch expected")
    };
    assert_eq!(snapshot_label, "My stick · Device 1");
    assert_eq!(entries.len(), 5);
    assert_eq!(f.completed.get(), 5);
    assert_eq!(
        f.state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .mappings()
            .len(),
        1
    );
}
#[test]
fn queued_delete_then_batch_readd_retains_membership_acknowledgements() {
    use inputforge_gui_dx::mapping_rail::{Lookup, Operation, Outcome, Request, Scope};
    let (_dom, f, rx) = mount();
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&address(0), "Default", None, vec![]);
    let scope = Scope {
        profile: f
            .state
            .read()
            .active_profile
            .as_ref()
            .map(|p| p.id().clone()),
        mode: "Default".into(),
    };
    let menu =
        f.c.rail_menu(Lookup {
            scope,
            input: address(0),
            name: None,
            filtered: false,
        })
        .unwrap();
    assert_eq!(
        f.c.rail(Request {
            target: menu.target,
            operation: Operation::Delete
        }),
        Outcome::Delivered
    );
    f.act(Intent::Apply);
    let EngineCommand::RemoveMapping { input, mode } = rx.try_recv().unwrap() else {
        panic!("delete expected")
    };
    f.state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&input, &mode);
    let EngineCommand::MappingEditBarrier { owner, serial } = rx.try_recv().unwrap() else {
        panic!("barrier expected")
    };
    f.state.write().mapping_edit_barrier = Some((owner, serial));
    f.refresh();
    assert_eq!(f.model().counts.skip, 5);
    assert_eq!(commands(&f.state, &rx).len(), 5);
    f.refresh();
    f.act(Intent::Apply);
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
