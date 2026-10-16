use super::*;
#[test]
fn stale_profile_and_offline_actions_are_rejected() {
    let (c, s, rx) = fixture(vec![]);
    let r = request(
        &s,
        Operation::Add {
            raw: "Approach".into(),
            blur: false,
        },
    );
    s.write().active_profile = Some(Profile::new(
        "P".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    c.mode_action(r).unwrap_err();
    s.write().session.offline = true;
    c.mode_action(request(&s, Operation::BeginAdd)).unwrap_err();
    rx.try_recv().unwrap_err();
}
#[test]
fn successful_delete_uses_survivor_focus_and_rename_invalidates_only_dependencies() {
    let (c, s, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        replace(&s, mapping(vec![Action::Invert]), 0, value(0.5)),
        now()
    ));
    apply(&s, &rx);
    s.write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_modes(Modes::new(vec!["Default".into(), "Combat".into(), "Landing".into()]).unwrap());
    let Ok(Transition::Delete(t)) =
        c.mode_action(request(&s, Operation::BeginDelete("Combat".into())))
    else {
        panic!("delete confirmation")
    };
    assert!(
        matches!(c.mode_action(request(&s,Operation::Delete(t))),Ok(Transition::Close(Some(n))) if n == "Landing")
    );
    assert_eq!(c.view().stacks[&key()].undo.len(), 1);
    assert!(
        matches!(rx.try_recv().unwrap(),EngineCommand::DeleteMode { name } if name == "Combat")
    );
}
#[test]
fn cancellation_survives_profile_replacement_and_offline_state() {
    let (c, s, rx) = fixture(vec![]);
    let r = request(&s, Operation::Cancel(Some("Default".into())));
    s.write().active_profile = None;
    s.write().session.offline = true;
    assert!(matches!(c.mode_action(r), Ok(Transition::Close(None))));
    rx.try_recv().unwrap_err();
}
#[test]
fn recreated_mode_revision_rejects_old_delete_and_rename_workflows() {
    let (c, s, rx) = fixture(vec![]);
    let deletion = request(&s, Operation::BeginDelete("Other".into()));
    let Ok(Transition::Delete(target)) = c.mode_action(deletion.clone()) else {
        panic!("confirmation")
    };
    let rename = request(
        &s,
        Operation::Rename {
            from: "Other".into(),
            raw: "Landing".into(),
            blur: false,
        },
    );
    s.write().mode_revision += 2;
    let stale = Request {
        scope: deletion.scope,
        operation: Operation::Delete(target),
    };
    c.mode_action(stale).unwrap_err();
    c.mode_action(rename).unwrap_err();
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
