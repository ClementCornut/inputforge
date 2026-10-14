use super::*;
use inputforge_core::{action::Action, engine::EngineCommand};
use inputforge_gui_dx::mapping_rail::{Operation, Outcome, Request};
#[test]
fn duplicate_to_mode_preserves_optional_name_and_latest_queued_actions() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let t = target(&state);
    assert!(c.dispatch(replace(&state, t.before.clone(), 0, value(0.5)), now()));
    let EngineCommand::SetMapping {
        actions: expected, ..
    } = rx.try_recv().unwrap()
    else {
        panic!("edit expected")
    };
    rx.try_recv().unwrap();
    assert_eq!(
        c.rail(Request {
            target: t,
            operation: Operation::Duplicate("Other".into())
        }),
        Outcome::Delivered
    );
    assert!(
        matches!(rx.try_recv().unwrap(), EngineCommand::SetMapping { name: None, mode, actions, .. } if mode == "Other" && actions == expected)
    );
    assert_eq!(c.view().stacks[&key()].undo.len(), 1);
}
#[test]
fn stale_delete_rejects_replacement_and_failed_delivery_preserves_history() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let t = target(&state);
    assert!(c.dispatch(replace(&state, t.before.clone(), 0, value(0.5)), now()));
    let current = apply(&state, &rx);
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &key().1,
        "Default",
        Some("External".into()),
        current.actions,
    );
    assert!(matches!(
        c.rail(Request {
            target: t,
            operation: Operation::Delete
        }),
        Outcome::Rejected(_)
    ));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, value(0.5)),
        now()
    ));
    let before = apply(&state, &rx);
    drop(rx);
    assert!(matches!(
        c.rail(Request {
            target: Target {
                scope: scope(&state),
                before
            },
            operation: Operation::Delete
        }),
        Outcome::Rejected(_)
    ));
    assert_eq!(c.view().stacks[&key()].undo.len(), 1);
}
#[test]
fn delete_invalidates_history_after_delivery_and_queued_delete_cannot_be_duplicated() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, value(0.5)),
        now()
    ));
    let before = apply(&state, &rx);
    let t = Target {
        scope: scope(&state),
        before,
    };
    assert_eq!(
        c.rail(Request {
            target: t.clone(),
            operation: Operation::Delete
        }),
        Outcome::Delivered
    );
    assert_eq!(c.view().stacks.len(), 0);
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::RemoveMapping { .. }
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::MappingEditBarrier { .. }
    ));
    assert!(matches!(
        c.rail(Request {
            target: t,
            operation: Operation::Duplicate("Other".into())
        }),
        Outcome::Rejected(_)
    ));
}
#[test]
fn invalid_modes_and_offline_operations_deliver_nothing() {
    for offline in [false, true] {
        let (c, state, rx) = fixture(vec![Action::Invert]);
        state.write().session.offline = offline;
        assert!(matches!(
            c.rail(Request {
                target: target(&state),
                operation: Operation::Duplicate("Missing".into())
            }),
            Outcome::Rejected(_)
        ));
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    }
}
