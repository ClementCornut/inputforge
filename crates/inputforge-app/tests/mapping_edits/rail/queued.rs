use super::*;
use inputforge_core::{action::Action, engine::EngineCommand};
use inputforge_gui_dx::mapping_rail::{Group, Move, Operation, Outcome, Request};
#[test]
fn queued_reorder_does_not_reuse_old_positions_until_acknowledged() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, "Default");
    for i in 0..3 {
        state.write().active_profile.as_mut().unwrap().set_mapping(
            &address(i),
            "Default",
            None,
            vec![],
        );
    }
    let m = c
        .rail_menu(Lookup {
            scope: scope(&state),
            input: address(1),
            name: None,
            filtered: false,
        })
        .unwrap();
    let request = Request {
        target: m.target,
        operation: Operation::Reorder {
            movement: Move::Down,
            order: m.order,
            group: Group::Buttons,
            filtered: false,
        },
    };
    assert!(matches!(
        c.rail(request.clone()),
        Outcome::Moved { index: 2, .. }
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::ReorderMapping { .. }
    ));
    assert!(matches!(c.rail(request.clone()), Outcome::NoOp));
    let EngineCommand::MappingEditBarrier { owner, serial } = rx.try_recv().unwrap() else {
        panic!("reorder acknowledgement expected")
    };
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .reorder_mapping_in_group(&address(1), "Default", 2);
    state.write().mapping_edit_barrier = Some((owner, serial));
    assert!(matches!(c.rail(request), Outcome::Rejected(_)));
}
#[test]
fn queued_deletion_cannot_shift_another_reorder_target() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, "Default");
    for i in 0..3 {
        state.write().active_profile.as_mut().unwrap().set_mapping(
            &address(i),
            "Default",
            None,
            vec![],
        );
    }
    let m = c
        .rail_menu(Lookup {
            scope: scope(&state),
            input: address(1),
            name: None,
            filtered: false,
        })
        .unwrap();
    let delete = c
        .rail_menu(Lookup {
            scope: scope(&state),
            input: address(0),
            name: None,
            filtered: false,
        })
        .unwrap();
    assert_eq!(
        c.rail(Request {
            target: delete.target,
            operation: Operation::Delete
        }),
        Outcome::Delivered
    );
    rx.try_recv().unwrap();
    rx.try_recv().unwrap();
    assert_eq!(
        c.rail(Request {
            target: m.target,
            operation: Operation::Reorder {
                movement: Move::Down,
                order: m.order,
                group: Group::Buttons,
                filtered: false
            }
        }),
        Outcome::NoOp
    );
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}

#[test]
fn remove_then_readd_cannot_hide_the_pending_membership_change() {
    use inputforge_gui_dx::mapping_rail::{AddIntent, AddRequest, AddView};
    let (c, state, rx) = fixture(vec![Action::Invert]);
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, "Default");
    for i in 0..3 {
        state.write().active_profile.as_mut().unwrap().set_mapping(
            &address(i),
            "Default",
            None,
            vec![],
        );
    }
    let m = c
        .rail_menu(Lookup {
            scope: scope(&state),
            input: address(1),
            name: None,
            filtered: false,
        })
        .unwrap();
    let deleted = c
        .rail_menu(Lookup {
            scope: scope(&state),
            input: address(0),
            name: None,
            filtered: false,
        })
        .unwrap();
    assert_eq!(
        c.rail(Request {
            target: deleted.target,
            operation: Operation::Delete
        }),
        Outcome::Delivered
    );
    rx.try_recv().unwrap();
    rx.try_recv().unwrap();
    let v = c.add(AddRequest {
        view: AddView::default(),
        intent: AddIntent::Start {
            scope: scope(&state),
            source: None,
        },
    });
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Captured(address(0)),
    });
    let _ = c.add(AddRequest {
        view: v,
        intent: AddIntent::Commit(String::new()),
    });
    rx.try_recv().unwrap();
    rx.try_recv().unwrap();
    assert_eq!(
        c.rail(Request {
            target: m.target,
            operation: Operation::Reorder {
                movement: Move::Down,
                order: m.order,
                group: Group::Buttons,
                filtered: false
            }
        }),
        Outcome::NoOp
    );
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
