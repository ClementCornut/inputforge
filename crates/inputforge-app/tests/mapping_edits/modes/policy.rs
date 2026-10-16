use super::*;
#[test]
fn mode_commands_use_current_core_validation_and_do_not_mutate_state() {
    let (c, s, rx) = fixture(vec![Action::Invert]);
    for raw in ["", "  ", "default", &"x".repeat(65)] {
        c.mode_action(request(
            &s,
            Operation::Add {
                raw: raw.into(),
                blur: false,
            },
        ))
        .unwrap_err();
        rx.try_recv().unwrap_err();
    }
    assert!(
        matches!(c.mode_action(request(&s, Operation::Add { raw: "  Approach  ".into(), blur: false })), Ok(Transition::Close(Some(n))) if n == "Approach")
    );
    assert!(
        matches!(rx.try_recv().unwrap(), EngineCommand::AddMode { name } if name == "Approach")
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::MappingEditBarrier { .. }
    ));
    assert!(
        !s.read()
            .active_profile
            .as_ref()
            .unwrap()
            .modes()
            .contains("Approach")
    );
}
#[test]
fn delete_revalidates_guards_and_confirmation_count() {
    let (c, s, rx) = fixture(vec![]);
    c.mode_action(request(&s, Operation::BeginDelete("Default".into())))
        .unwrap_err();
    s.write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_modes(Modes::new(vec!["Default".into(), "Combat".into(), "Landing".into()]).unwrap());
    let Ok(Transition::Delete(target)) =
        c.mode_action(request(&s, Operation::BeginDelete("Combat".into())))
    else {
        panic!("confirmation expected")
    };
    s.write().active_profile.as_mut().unwrap().set_mapping(
        &inputforge_core::types::InputAddress::Unbound,
        "Combat",
        None,
        vec![],
    );
    c.mode_action(request(&s, Operation::Delete(target)))
        .unwrap_err();
    s.write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_startup_mode("Combat".into());
    c.mode_action(request(&s, Operation::BeginDelete("Combat".into())))
        .unwrap_err();
    rx.try_recv().unwrap_err();
}
#[test]
fn rename_noop_and_empty_blur_do_not_dispatch() {
    let (c, s, rx) = fixture(vec![]);
    assert!(matches!(
        c.mode_action(request(
            &s,
            Operation::Rename {
                from: "Default".into(),
                raw: " Default ".into(),
                blur: false
            }
        )),
        Ok(Transition::Close(None))
    ));
    assert!(matches!(
        c.mode_action(request(
            &s,
            Operation::Add {
                raw: "  ".into(),
                blur: true
            }
        )),
        Ok(Transition::Close(None))
    ));
    rx.try_recv().unwrap_err();
}
#[test]
fn rename_preserves_unicode_and_blocks_second_mutation_until_queue_acknowledgement() {
    let (c, s, rx) = fixture(vec![]);
    let name = "👨‍👩‍👧‍👦".repeat(64);
    c.mode_action(request(
        &s,
        Operation::Rename {
            from: "Default".into(),
            raw: name.clone(),
            blur: false,
        },
    ))
    .unwrap();
    assert!(matches!(rx.try_recv().unwrap(),EngineCommand::RenameMode {to,..} if to==name));
    c.mode_action(request(
        &s,
        Operation::Add {
            raw: "Other".into(),
            blur: false,
        },
    ))
    .unwrap_err();
}
#[test]
fn projection_exposes_current_guards_counts_and_runtime_marker() {
    let (_, s, _) = fixture(vec![Action::Invert]);
    s.write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_modes(Modes::new(vec!["Default".into(), "Combat".into(), "Landing".into()]).unwrap());
    s.write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_startup_mode("Combat".into());
    s.write().current_mode = "Landing".into();
    let rows = inputforge_app::bridge::prepare(&s.read(), None)
        .meta
        .mode_management
        .rows;
    assert!(!rows[0].can_delete);
    assert!(!rows[1].can_delete);
    assert!(rows[2].can_delete);
    assert_eq!(rows[0].mappings, 1);
    assert_eq!(rows[1].mappings, 0);
    assert!(rows[2].running);
    assert!(!rows[2].can_activate);
    assert!(!rows[1].can_default);
}
#[test]
fn deletion_confirmation_counts_a_mapping_queued_before_the_next_poll() {
    use inputforge_gui_dx::mapping_rail as rail;
    let (c, s, _rx) = fixture(vec![Action::Invert]);
    let profile = s.read().active_profile.as_ref().unwrap().id().clone();
    assert_eq!(
        c.rail(rail::Request {
            target: rail::Target {
                scope: rail::Scope {
                    profile: Some(profile),
                    mode: "Default".into()
                },
                before: mapping(vec![Action::Invert])
            },
            operation: rail::Operation::Duplicate("Other".into())
        }),
        rail::Outcome::Delivered
    );
    let Ok(Transition::Delete(t)) =
        c.mode_action(request(&s, Operation::BeginDelete("Other".into())))
    else {
        panic!("confirmation")
    };
    assert_eq!(
        t.mappings, 1,
        "queued mapping would be deleted without being counted"
    );
}
#[test]
fn removed_editing_mode_cannot_authorize_an_add() {
    let (c, s, rx) = fixture(vec![]);
    let mut r = request(
        &s,
        Operation::Add {
            raw: "Approach".into(),
            blur: false,
        },
    );
    r.scope.editing = "Missing".into();
    c.mode_action(r).unwrap_err();
    rx.try_recv().unwrap_err();
}
