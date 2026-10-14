use super::support::*;
use inputforge_core::action::{Action, ActionBranch, Condition};
use inputforge_gui_dx::mapping_edits::{
    Edit, StageId,
    StageIdSegment::{Branch, Index},
};

#[test]
fn delivered_edits_record_after_dispatch_without_mutating_engine() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    assert!(c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, value(0.5)),
        now()
    ));
    assert_eq!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&key().1, &key().0)
            .unwrap()
            .actions,
        vec![Action::Invert]
    );
    assert_eq!(c.view().stacks[&key()].undo.len(), 1);
    assert_eq!(apply(&state, &rx).name, None);
}
#[test]
fn disconnected_delivery_does_not_create_history() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    drop(rx);
    assert!(!c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, Action::Invert),
        now()
    ));
    assert_eq!(c.view().stacks.len(), 0);
}
#[test]
fn stale_profile_name_tree_mode_and_path_are_rejected() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let valid = replace(&state, mapping(vec![Action::Invert]), 0, Action::Invert);
    let mut cases = vec![];
    let mut r = valid.clone();
    r.profile = None;
    cases.push(r);
    let mut r = valid.clone();
    r.before.name = Some("wrong".into());
    cases.push(r);
    let mut r = valid.clone();
    r.before.actions.clear();
    cases.push(r);
    let mut r = valid.clone();
    r.before.mode = "missing".into();
    cases.push(r);
    let mut r = valid.clone();
    r.edit = Edit::Replace {
        path: stage(9),
        action: Action::Invert,
    };
    cases.push(r);
    for r in cases {
        assert!(!c.dispatch(r, now()));
    }
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(c.view().stacks.len(), 0);
}
#[test]
fn nested_edits_preserve_sibling_branches_and_invalid_shapes_dispatch_nothing() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: key().1 },
        if_true: vec![Action::Invert],
        if_false: vec![Action::Invert],
    }];
    let (c, state, rx) = fixture(actions.clone());
    let path = StageId(vec![
        Index(0),
        Branch(ActionBranch::ConditionalTrue),
        Index(0),
    ]);
    assert!(c.dispatch(
        request(
            &state,
            mapping(actions),
            Edit::Replace {
                path,
                action: value(1.0)
            }
        ),
        now()
    ));
    let result = apply(&state, &rx);
    assert!(
        matches!(&result.actions[0], Action::Conditional { if_true, if_false, .. } if matches!(if_true[0], Action::ResponseCurve { .. }) && *if_false == vec![Action::Invert])
    );
    for path in [
        StageId(vec![]),
        StageId(vec![Branch(ActionBranch::ConditionalTrue)]),
        StageId(vec![Index(0), Index(0)]),
        StageId(vec![Index(0), Branch(ActionBranch::TapSingle), Index(0)]),
    ] {
        assert!(!c.dispatch(
            request(
                &state,
                result.clone(),
                Edit::Replace {
                    path,
                    action: Action::Invert
                }
            ),
            now()
        ));
    }
}
#[test]
fn rapid_edits_before_polling_preserve_previous_delivered_stages() {
    let initial = mapping(vec![Action::Invert, Action::Invert]);
    let (c, state, rx) = fixture(initial.actions.clone());
    assert!(c.dispatch(replace(&state, initial.clone(), 0, value(1.0)), now()));
    assert!(c.dispatch(replace(&state, initial, 1, value(0.5)), now()));
    apply(&state, &rx);
    let final_mapping = apply(&state, &rx);
    assert_eq!(final_mapping.actions, vec![value(1.0), value(0.5)]);
}

#[test]
fn structural_pending_edits_reject_old_positional_targets() {
    let original = mapping(vec![value(0.25), value(0.5), value(0.75)]);
    let (c, state, rx) = fixture(original.actions.clone());
    assert!(c.dispatch(
        request(&state, original.clone(), Edit::Remove(stage(0))),
        now()
    ));
    assert!(!c.dispatch(request(&state, original, Edit::Remove(stage(1))), now()));
    assert_eq!(apply(&state, &rx).actions, vec![value(0.5), value(0.75)]);
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn core_rejected_action_trees_never_dispatch_or_enter_history() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let invalid = Action::TapGesture {
        threshold_ms: 0,
        fire_single_immediately: false,
        single_tap: vec![],
        double_tap: vec![],
    };
    assert!(!c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, invalid),
        now()
    ));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(c.view().stacks.len(), 0);
}

#[test]
fn tap_and_press_toggle_after_local_threshold_edit_restore_the_delivered_threshold() {
    for tap in [true, false] {
        let build = |threshold, toggle| {
            if tap {
                Action::TapGesture {
                    threshold_ms: threshold,
                    fire_single_immediately: toggle,
                    single_tap: vec![Action::Invert],
                    double_tap: vec![],
                }
            } else {
                Action::PressGesture {
                    threshold_ms: threshold,
                    fire_long_when_threshold_crossed: toggle,
                    short_press: vec![Action::Invert],
                    long_press: vec![],
                }
            }
        };
        let (c, state, rx) = fixture(vec![build(250, false)]);
        let initial = bind(
            &state,
            inputforge_core::types::InputAddress::Bound {
                device: inputforge_core::types::DeviceId("fixture".into()),
                input: inputforge_core::types::InputId::Button { index: 0 },
            },
        );
        let owner = (initial.mode.clone(), initial.input.clone());
        assert!(c.dispatch(
            replace(&state, initial.clone(), 0, build(750, false)),
            now()
        ));
        assert!(c.dispatch(replace(&state, initial, 0, build(750, true)), now()));
        apply(&state, &rx);
        assert_eq!(apply(&state, &rx).actions, vec![build(750, true)]);
        assert!(c.undo(&owner));
        assert_eq!(apply(&state, &rx).actions, vec![build(750, false)]);
    }
}

#[test]
fn stale_parent_replacement_cannot_discard_a_queued_nested_edit() {
    let parent = Action::Conditional {
        condition: Condition::ButtonPressed { input: key().1 },
        if_true: vec![Action::Invert],
        if_false: vec![],
    };
    let initial = mapping(vec![parent.clone()]);
    let (c, state, rx) = fixture(initial.actions.clone());
    let path = StageId(vec![
        Index(0),
        Branch(ActionBranch::ConditionalTrue),
        Index(0),
    ]);
    assert!(c.dispatch(
        request(
            &state,
            initial.clone(),
            Edit::Replace {
                path,
                action: value(0.5)
            }
        ),
        now()
    ));
    let mut changed_parent = parent;
    if let Action::Conditional { condition, .. } = &mut changed_parent {
        *condition = Condition::ButtonReleased { input: key().1 };
    }
    assert!(!c.dispatch(replace(&state, initial, 0, changed_parent), now()));
    assert!(
        matches!(&apply(&state, &rx).actions[0], Action::Conditional { if_true, .. } if if_true == &vec![value(0.5)])
    );
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}

#[test]
fn disjoint_edits_survive_engine_delivery_before_the_renderer_refreshes() {
    let initial = mapping(vec![Action::Invert, Action::Invert]);
    let (c, state, rx) = fixture(initial.actions.clone());
    assert!(c.dispatch(replace(&state, initial.clone(), 0, value(0.25)), now()));
    apply(&state, &rx);
    assert!(c.dispatch(replace(&state, initial, 1, value(0.5)), now()));
    assert_eq!(apply(&state, &rx).actions, vec![value(0.25), value(0.5)]);
}

#[test]
fn restore_round_trip_rejects_a_stale_pre_restore_baseline() {
    let original = mapping(vec![Action::Invert, Action::Invert]);
    let (c, state, rx) = fixture(original.actions.clone());
    assert!(c.dispatch(replace(&state, original.clone(), 0, value(0.25)), now()));
    let first = apply(&state, &rx);
    assert!(c.dispatch(replace(&state, first, 0, value(0.5)), now()));
    apply(&state, &rx);
    assert!(c.undo(&key()));
    apply(&state, &rx);
    assert!(c.redo(&key()));
    apply(&state, &rx);
    assert!(!c.dispatch(replace(&state, original, 1, value(0.75)), now()));
}
#[test]
fn rebind_revalidates_gesture_shape_for_the_destination_input() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    let (c, state, rx) = fixture(vec![Action::TapGesture {
        threshold_ms: 250,
        fire_single_immediately: false,
        single_tap: vec![],
        double_tap: vec![],
    }]);
    let before = bind(
        &state,
        InputAddress::Bound {
            device: DeviceId("fixture".into()),
            input: InputId::Button { index: 0 },
        },
    );
    let destination = InputAddress::Bound {
        device: DeviceId("fixture".into()),
        input: InputId::Axis { index: 0 },
    };
    assert!(!c.dispatch(request(&state, before, Edit::Rebind(destination)), now()));
    assert!(c.view().stacks.is_empty());
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
