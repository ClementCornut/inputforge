use super::*;
use inputforge_core::{action::Action, engine::EngineCommand};
use inputforge_gui_dx::mapping_rail::{Group, Move, Operation, Outcome, Request};
#[test]
fn gaps_convert_after_removal_and_reject_adjacent_filtered_or_stale_orders() {
    for (gap, expected) in [(0, Some(0)), (1, None), (2, None), (3, Some(2))] {
        let (c, state, rx) = fixture(vec![Action::Invert]);
        for i in 0..3 {
            state.write().active_profile.as_mut().unwrap().set_mapping(
                &address(i),
                "Default",
                None,
                vec![],
            );
        }
        let before = inputforge_core::action::Mapping {
            input: address(1),
            mode: "Default".into(),
            name: None,
            actions: vec![],
        };
        let order = vec![address(0), address(1), address(2)];
        let request = Request {
            target: Target {
                scope: scope(&state),
                before,
            },
            operation: Operation::Reorder {
                movement: Move::Gap(gap),
                order,
                group: Group::Buttons,
                filtered: false,
            },
        };
        let result = c.rail(request.clone());
        if let Some(index) = expected {
            assert_eq!(
                result,
                Outcome::Moved {
                    index,
                    len: 3,
                    group: Group::Buttons
                }
            );
            assert!(
                matches!(rx.try_recv().unwrap(), EngineCommand::ReorderMapping { input, target_index_in_group, .. } if input == address(1) && target_index_in_group == index)
            );
            let EngineCommand::MappingEditBarrier { owner, serial } = rx.try_recv().unwrap() else {
                panic!("barrier expected")
            };
            state.write().mapping_edit_barrier = Some((owner, serial));
        } else {
            assert_eq!(result, Outcome::NoOp);
        }
        let mut obsolete = request.clone();
        if let Operation::Reorder { order, .. } = &mut obsolete.operation {
            order.swap(0, 2);
        }
        assert!(matches!(c.rail(obsolete), Outcome::Rejected(_)));
        let mut filtered = request;
        if let Operation::Reorder { filtered, .. } = &mut filtered.operation {
            *filtered = true;
        }
        assert_eq!(c.rail(filtered), Outcome::NoOp);
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    }
}
#[test]
fn boundary_and_cross_group_moves_are_noops_without_history_loss() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, value(0.5)),
        now()
    ));
    let before = apply(&state, &rx);
    for group in [Group::Buttons, Group::Axes] {
        let r = Request {
            target: Target {
                scope: scope(&state),
                before: before.clone(),
            },
            operation: Operation::Reorder {
                movement: Move::Up,
                order: vec![key().1],
                group,
                filtered: false,
            },
        };
        assert!(matches!(c.rail(r), Outcome::NoOp | Outcome::Rejected(_)));
    }
    assert_eq!(c.view().stacks[&key()].undo.len(), 1);
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
