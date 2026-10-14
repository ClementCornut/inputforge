use super::support::*;
use inputforge_core::action::Action;
use inputforge_gui_dx::mapping_edits::Edit;

#[test]
fn repeated_undo_redo_restores_distinct_snapshots_and_optional_names() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        request(
            &state,
            mapping(vec![Action::Invert]),
            Edit::Rename("Named".into())
        ),
        now()
    ));
    let renamed = apply(&state, &rx);
    assert!(c.dispatch(replace(&state, renamed, 0, value(1.0)), now()));
    let edited = apply(&state, &rx);
    for _ in 0..2 {
        assert!(c.undo(&key()));
        assert_eq!(apply(&state, &rx).actions, vec![Action::Invert]);
        assert!(c.undo(&key()));
        assert_eq!(apply(&state, &rx).name, None);
        assert!(c.redo(&key()));
        assert_eq!(apply(&state, &rx).name.as_deref(), Some("Named"));
        assert!(c.redo(&key()));
        assert_eq!(apply(&state, &rx), edited);
    }
}
#[test]
fn failed_undo_and_redo_leave_stacks_unchanged() {
    for redo in [false, true] {
        let (c, state, rx) = fixture(vec![Action::Invert]);
        assert!(c.dispatch(
            replace(&state, mapping(vec![Action::Invert]), 0, value(1.0)),
            now()
        ));
        apply(&state, &rx);
        if redo {
            assert!(c.undo(&key()));
            apply(&state, &rx);
        }
        let before = c.view();
        drop(rx);
        assert!(!(if redo { c.redo(&key()) } else { c.undo(&key()) }));
        assert_eq!(c.view(), before);
    }
}
#[test]
fn last_stage_removal_can_be_undone_and_redone() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        request(
            &state,
            mapping(vec![Action::Invert]),
            Edit::Remove(stage(0))
        ),
        now()
    ));
    assert_eq!(apply(&state, &rx).actions.len(), 0);
    assert!(c.undo(&key()));
    assert_eq!(apply(&state, &rx).actions, vec![Action::Invert]);
    assert!(c.redo(&key()));
    assert_eq!(apply(&state, &rx).actions.len(), 0);
}
#[test]
fn histories_are_isolated_by_mode_and_profile_identity() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let mut other = mapping(vec![Action::Invert]);
    other.mode = "Other".into();
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &other.input,
        &other.mode,
        other.name.clone(),
        other.actions.clone(),
    );
    assert!(c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, value(1.0)),
        now()
    ));
    apply(&state, &rx);
    assert!(c.dispatch(replace(&state, other, 0, value(0.5)), now()));
    apply(&state, &rx);
    assert!(c.undo(&key()));
    apply(&state, &rx);
    assert_eq!(c.view().stacks[&("Other".into(), key().1)].undo.len(), 1);
    let (_, replacement, _) = fixture(vec![Action::Invert]);
    state.write().active_profile = replacement.read().active_profile.clone();
    assert!(!c.redo(&key()));
    assert_eq!(c.view().stacks.len(), 0);
}
#[test]
fn external_changes_invalidate_history_without_resurrecting_removed_mappings() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, value(1.0)),
        now()
    ));
    apply(&state, &rx);
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, &key().0);
    assert!(!c.undo(&key()));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn cap_and_fresh_edits_preserve_fifo_and_clear_redo() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let mut before = mapping(vec![Action::Invert]);
    for i in 0..60 {
        let mut r = request(&state, before, Edit::Rename(format!("name{i}")));
        r.label = format!("rename{i}");
        assert!(c.dispatch(r, now()));
        before = apply(&state, &rx);
    }
    assert_eq!(c.view().stacks[&key()].undo.len(), 50);
    assert_eq!(c.view().stacks[&key()].undo[0].label, "rename10");
    assert!(c.undo(&key()));
    let before = apply(&state, &rx);
    assert!(c.dispatch(request(&state, before, Edit::Rename("fresh".into())), now()));
    assert_eq!(c.view().stacks[&key()].redo.len(), 0);
}

#[test]
fn delivered_round_trip_does_not_hide_later_external_deletion() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        request(
            &state,
            mapping(vec![Action::Invert]),
            Edit::Remove(stage(0))
        ),
        now()
    ));
    assert!(c.undo(&key()));
    apply(&state, &rx);
    apply(&state, &rx);
    // Drive reconciliation once while the engine has returned to the original state.
    let mut invalid = replace(&state, mapping(vec![Action::Invert]), 9, Action::Invert);
    invalid.profile = None;
    assert!(!c.dispatch(invalid, now()));
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, &key().0);
    assert!(!c.redo(&key()));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}

#[test]
fn another_mapping_cannot_restore_over_a_newer_destination_edit() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let dest = InputAddress::Bound {
        device: DeviceId("dev".into()),
        input: InputId::Button { index: 1 },
    };
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &dest,
        "Default",
        None,
        vec![value(0.75)],
    );
    assert!(c.dispatch(
        request(
            &state,
            mapping(vec![Action::Invert]),
            Edit::Rebind(dest.clone())
        ),
        now()
    ));
    let rebound = apply(&state, &rx);
    assert!(c.dispatch(replace(&state, rebound, 0, value(0.5)), now()));
    apply(&state, &rx);
    assert!(!c.undo(&key()));
    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&dest, "Default")
            .unwrap()
            .actions,
        vec![value(0.5)]
    );
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn empty_mapping_restore_preserves_its_saved_optional_name() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        request(
            &state,
            mapping(vec![Action::Invert]),
            Edit::Rename("empty row".into())
        ),
        now()
    ));
    let named = apply(&state, &rx);
    assert!(c.dispatch(request(&state, named, Edit::Remove(stage(0))), now()));
    assert_eq!(apply(&state, &rx).name.as_deref(), Some("empty row"));
    assert!(c.undo(&key()));
    apply(&state, &rx);
    assert!(c.redo(&key()));
    assert_eq!(apply(&state, &rx).name.as_deref(), Some("empty row"));
}

#[test]
fn confirmed_round_trip_rejects_an_external_change_to_an_old_queued_value() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let mut initial = mapping(vec![Action::Invert]);
    initial.name = Some("A".into());
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &initial.input,
        &initial.mode,
        initial.name.clone(),
        initial.actions.clone(),
    );
    assert!(c.dispatch(
        request(&state, initial.clone(), Edit::Rename("B".into())),
        now()
    ));
    assert!(c.undo(&key()));
    // An engine sample between the two edits must retain the in-flight history.
    let inputforge_core::engine::EngineCommand::SetMapping {
        input,
        mode,
        name,
        actions,
    } = rx.try_recv().unwrap()
    else {
        panic!("rename expected")
    };
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&input, &mode, name, actions);
    let mut invalid = request(&state, initial.clone(), Edit::Rename("ignored".into()));
    invalid.profile = None;
    assert!(!c.dispatch(invalid.clone(), now()));
    assert_eq!(c.view().stacks[&key()].redo.len(), 1);
    let inputforge_core::engine::EngineCommand::MappingEditBarrier { owner, serial } =
        rx.try_recv().unwrap()
    else {
        panic!("barrier expected")
    };
    state.write().mapping_edit_barrier = Some((owner, serial));
    apply(&state, &rx);
    assert!(!c.dispatch(invalid, now()));
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &key().1,
        &key().0,
        Some("B".into()),
        vec![Action::Invert],
    );
    assert!(!c.redo(&key()));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}

#[test]
fn deleting_a_rebind_source_invalidates_its_destination_history() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let dest = InputAddress::Bound {
        device: DeviceId("dev".into()),
        input: InputId::Button { index: 1 },
    };
    assert!(c.dispatch(
        request(
            &state,
            mapping(vec![Action::Invert]),
            Edit::Rebind(dest.clone())
        ),
        now()
    ));
    apply(&state, &rx);
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, &key().0);
    assert!(!c.undo(&key()));
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&dest, "Default")
            .is_some()
    );
}
#[test]
fn writer_deletion_during_dispatch_or_restore_cannot_resurrect_a_mapping() {
    for restore in [false, true] {
        let (c, state, rx) = fixture(vec![Action::Invert]);
        assert!(c.dispatch(
            replace(&state, mapping(vec![Action::Invert]), 0, value(0.25)),
            now()
        ));
        let delivered = apply(&state, &rx);
        let edit = replace(&state, delivered, 0, value(0.5));
        let writer_state = std::sync::Arc::clone(&state);
        let (locked, ready) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            let mut state = writer_state.write();
            state
                .active_profile
                .as_mut()
                .unwrap()
                .remove_mapping(&key().1, &key().0);
            locked.send(()).unwrap();
            // Keep the writer active through the nonblocking poll and operation's read acquisition.
            std::thread::sleep(std::time::Duration::from_millis(100));
        });
        ready.recv().unwrap();
        let accepted = if restore {
            c.undo(&key())
        } else {
            c.dispatch(edit, now())
        };
        writer.join().unwrap();
        assert!(!accepted);
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
        assert!(
            state
                .read()
                .active_profile
                .as_ref()
                .unwrap()
                .find_mapping(&key().1, &key().0)
                .is_none()
        );
        assert!(c.view().stacks.is_empty());
    }
}
