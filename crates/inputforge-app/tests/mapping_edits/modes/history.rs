use super::*;
#[test]
fn failed_delivery_produces_no_transition_or_history_change() {
    let (c, s, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        replace(&s, mapping(vec![Action::Invert]), 0, value(0.5)),
        now()
    ));
    apply(&s, &rx);
    let before = c.view();
    drop(rx);
    c.mode_action(request(
        &s,
        Operation::Rename {
            from: "Default".into(),
            raw: "Renamed".into(),
            blur: false,
        },
    ))
    .unwrap_err();
    assert_eq!(c.view(), before);
    assert!(
        s.read()
            .active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Default")
    );
}
#[test]
fn activate_and_startup_do_not_change_history_or_add_running_restrictions() {
    let (c, s, rx) = fixture(vec![]);
    s.write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_modes(Modes::new(vec!["Default".into(), "Combat".into()]).unwrap());
    s.write().engine_status = inputforge_core::state::EngineStatus::Running;
    c.mode_action(request(&s, Operation::Activate("Combat".into())))
        .unwrap();
    assert!(matches!(rx.try_recv().unwrap(),EngineCommand::SwitchMode {mode} if mode=="Combat"));
    c.mode_action(request(&s, Operation::Default("Combat".into())))
        .unwrap();
    assert!(
        matches!(rx.try_recv().unwrap(),EngineCommand::SetDefaultMode {name} if name=="Combat")
    );
    c.mode_action(request(&s, Operation::Activate("Missing".into())))
        .unwrap_err();
}
#[test]
fn rename_cascade_preserves_unrelated_history_and_invalidates_historical_mode_references() {
    let (c, s, rx) = fixture(vec![Action::ChangeMode {
        strategy: inputforge_core::action::ModeChangeStrategy::SwitchTo {
            mode: "Combat".into(),
        },
    }]);
    s.write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_modes(Modes::new(vec!["Default".into(), "Combat".into()]).unwrap());
    let before = mapping(vec![Action::ChangeMode {
        strategy: inputforge_core::action::ModeChangeStrategy::SwitchTo {
            mode: "Combat".into(),
        },
    }]);
    assert!(c.dispatch(replace(&s, before, 0, Action::Invert), now()));
    apply(&s, &rx);
    assert_eq!(c.view().stacks[&key()].undo.len(), 1);
    c.mode_action(request(
        &s,
        Operation::Rename {
            from: "Combat".into(),
            raw: "Landing".into(),
            blur: false,
        },
    ))
    .unwrap();
    assert!(
        !c.view().stacks.contains_key(&key()),
        "undo could restore an obsolete mode reference"
    );
}
#[test]
fn queued_mode_rename_rejects_a_duplicate_into_the_removed_name() {
    use inputforge_gui_dx::mapping_rail as rail;
    let (c, s, rx) = fixture(vec![Action::Invert]);
    c.mode_action(request(
        &s,
        Operation::Rename {
            from: "Other".into(),
            raw: "Landing".into(),
            blur: false,
        },
    ))
    .unwrap();
    let outcome = c.rail(rail::Request {
        target: rail::Target {
            scope: rail::Scope {
                profile: s.read().active_profile.as_ref().map(|p| p.id().clone()),
                mode: "Default".into(),
            },
            before: mapping(vec![Action::Invert]),
        },
        operation: rail::Operation::Duplicate("Other".into()),
    });
    assert!(
        matches!(outcome, rail::Outcome::Rejected(_)),
        "stale mode accepted: {outcome:?}"
    );
    rx.try_recv().unwrap();
    rx.try_recv().unwrap();
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn pending_mode_mutation_blocks_stage_edits_and_restores_until_acknowledged() {
    let (c, s, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        replace(&s, mapping(vec![Action::Invert]), 0, value(0.5)),
        now()
    ));
    let before = apply(&s, &rx);
    c.mode_action(request(
        &s,
        Operation::Rename {
            from: "Other".into(),
            raw: "Landing".into(),
            blur: false,
        },
    ))
    .unwrap();
    assert!(!c.dispatch(replace(&s, before.clone(), 0, value(0.75)), now()));
    assert!(!c.undo(&key()));
    let EngineCommand::RenameMode { from, to } = rx.try_recv().unwrap() else {
        panic!("rename")
    };
    {
        let mut state = s.write();
        let p = state.active_profile.as_mut().unwrap();
        p.rename_mode_refs(&from, &to);
        let modes = p.modes().with_renamed(&from, &to).unwrap();
        p.set_modes(modes);
        state.mode_revision += 1;
    };
    let EngineCommand::MappingEditBarrier { owner, serial } = rx.try_recv().unwrap() else {
        panic!("barrier")
    };
    s.write().mapping_edit_barrier = Some((owner, serial));
    assert!(c.dispatch(replace(&s, before, 0, value(0.75)), now()));
}
