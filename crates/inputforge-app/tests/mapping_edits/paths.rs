use super::support::*;
use inputforge_core::{
    action::{Action, ActionBranch, Condition, MouseTarget, OutputBehavior},
    processing::DeadzoneConfig,
    types::{DeviceId, InputAddress, InputId, MergeOp},
};
use inputforge_gui_dx::mapping_edits::{Edit, StageId, StageIdSegment};
fn synth_addr() -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("dev-1".into()),
        input: InputId::Button { index: 0 },
    }
}
fn tree(actions: &[Action], edit: Edit) -> Option<Vec<Action>> {
    let (controller, state, rx) = fixture(actions.to_vec());
    let before = bind(&state, synth_addr());
    controller
        .dispatch(request(&state, before, edit), now())
        .then(|| apply(&state, &rx).actions)
}
fn replace_at_path(actions: &[Action], path: &StageId, action: Action) -> Option<Vec<Action>> {
    tree(
        actions,
        Edit::Replace {
            path: path.clone(),
            action,
        },
    )
}
fn insert_at_path(actions: &[Action], path: &StageId, action: Action) -> Option<Vec<Action>> {
    tree(
        actions,
        Edit::Insert {
            path: path.clone(),
            action,
        },
    )
}
fn remove_at_path(actions: &[Action], path: &StageId) -> Option<Vec<Action>> {
    tree(actions, Edit::Remove(path.clone()))
}
#[test]
fn replace_at_path_outer_swaps_action() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    let new = replace_at_path(
        &actions,
        &path,
        Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
    )
    .expect("valid path must succeed");
    assert!(matches!(new[0], Action::MergeAxis { .. }));
}

#[test]
fn replace_at_path_inside_if_true_swaps_action() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![Action::Invert],
        if_false: Vec::new(),
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::ConditionalTrue),
        StageIdSegment::Index(0),
    ]);
    let new = replace_at_path(
        &actions,
        &path,
        Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
    )
    .expect("valid path must succeed");
    match &new[0] {
        Action::Conditional { if_true, .. } => {
            assert!(matches!(if_true[0], Action::MergeAxis { .. }));
        }
        _ => panic!("outer wrapper should remain Conditional"),
    }
}

#[test]
fn replace_at_path_invalid_path_returns_none() {
    // Out-of-range index: must return None, not panic, in BOTH debug
    // and release. Callers depend on this to skip the edit + skip
    // push_edit (no phantom undo entries).
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(99)]);
    assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

    // Empty path.
    let path = StageId(vec![]);
    assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

    // Path starts with a branch segment.
    let path = StageId(vec![StageIdSegment::Branch(ActionBranch::ConditionalTrue)]);
    assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

    // Branch segment after a non-Conditional action.
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::ConditionalTrue),
    ]);
    assert!(replace_at_path(&actions, &path, Action::Invert).is_none());
}

#[test]
fn insert_at_path_outer_appends() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(1)]);
    let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
    assert_eq!(new.len(), 2);
}

#[test]
fn insert_at_path_outer_inserts_at_index() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    let new = insert_at_path(
        &actions,
        &path,
        Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
    )
    .expect("valid path");
    assert_eq!(new.len(), 2);
    assert!(matches!(new[0], Action::MergeAxis { .. }));
    assert!(matches!(new[1], Action::Invert));
}

#[test]
fn insert_at_path_into_if_false_creates_branch() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![],
        if_false: Vec::new(),
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::ConditionalFalse),
        StageIdSegment::Index(0),
    ]);
    let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
    match &new[0] {
        Action::Conditional { if_false, .. } => {
            assert_eq!(if_false.len(), 1);
        }
        _ => panic!("expected Conditional"),
    }
}

#[test]
fn remove_at_path_outer_drops_action() {
    let actions = vec![Action::Invert, Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    let new = remove_at_path(&actions, &path).expect("valid path");
    assert_eq!(new.len(), 1);
}

#[test]
fn remove_at_path_last_in_if_false_leaves_empty_branch() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![],
        if_false: vec![Action::Invert],
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::ConditionalFalse),
        StageIdSegment::Index(0),
    ]);
    let new = remove_at_path(&actions, &path).expect("valid path");
    match &new[0] {
        Action::Conditional { if_false, .. } => {
            assert!(
                if_false.is_empty(),
                "removing the last action must leave an empty if_false branch"
            );
        }
        _ => panic!("expected Conditional"),
    }
}

#[test]
fn insert_remove_invalid_paths_return_none() {
    // Same contract as replace_at_path: callers depend on None
    // (NOT panic in release) so they can skip the edit + skip push_edit.
    let actions = vec![Action::Invert];

    // Empty path.
    assert!(insert_at_path(&actions, &StageId(vec![]), Action::Invert).is_none());
    assert!(remove_at_path(&actions, &StageId(vec![])).is_none());

    // Path starts with branch segment.
    assert!(
        insert_at_path(
            &actions,
            &StageId(vec![StageIdSegment::Branch(ActionBranch::ConditionalTrue)]),
            Action::Invert
        )
        .is_none()
    );
    assert!(
        remove_at_path(
            &actions,
            &StageId(vec![StageIdSegment::Branch(ActionBranch::ConditionalTrue)])
        )
        .is_none()
    );

    // Out-of-range index for remove_at_path.
    assert!(remove_at_path(&actions, &StageId(vec![StageIdSegment::Index(99)])).is_none());

    // Branch segment after a non-Conditional action.
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::ConditionalTrue),
        StageIdSegment::Index(0),
    ]);
    assert!(insert_at_path(&actions, &path, Action::Invert).is_none());
    assert!(remove_at_path(&actions, &path).is_none());
}

#[test]
fn branch_path_helpers_traverse_all_action_branch_variants() {
    let actions = vec![
        Action::TapGesture {
            threshold_ms: 250,
            fire_single_immediately: false,
            single_tap: vec![Action::Invert],
            double_tap: vec![Action::MergeAxis {
                second_input: synth_addr(),
                operation: MergeOp::Average,
            }],
        },
        Action::PressGesture {
            threshold_ms: 500,
            fire_long_when_threshold_crossed: false,
            short_press: vec![Action::MapToMouse {
                target: MouseTarget::LeftButton,
                behavior: OutputBehavior::Pulse,
            }],
            long_press: vec![Action::MapToMouse {
                target: MouseTarget::RightButton,
                behavior: OutputBehavior::Pulse,
            }],
        },
    ];

    let single_path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::TapSingle),
        StageIdSegment::Index(0),
    ]);
    assert!(matches!(
        at_path(&actions, &single_path),
        Some(Action::Invert)
    ));

    let double_path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::TapDouble),
        StageIdSegment::Index(0),
    ]);
    assert!(matches!(
        at_path(&actions, &double_path),
        Some(Action::MergeAxis { .. })
    ));

    let replaced = replace_at_path(&actions, &single_path, Action::Invert)
        .expect("tap single branch path must replace");
    assert!(matches!(
        at_path(&replaced, &single_path),
        Some(Action::Invert)
    ));

    let inserted = insert_at_path(&actions, &double_path, Action::Invert)
        .expect("tap double branch path must insert");
    match &inserted[0] {
        Action::TapGesture { double_tap, .. } => assert_eq!(double_tap.len(), 2),
        _ => panic!("outer wrapper should remain TapGesture"),
    }

    let removed =
        remove_at_path(&actions, &double_path).expect("tap double branch path must remove");
    match &removed[0] {
        Action::TapGesture { double_tap, .. } => assert_eq!(double_tap.len(), 0),
        _ => panic!("outer wrapper should remain TapGesture"),
    }

    let short_path = StageId(vec![
        StageIdSegment::Index(1),
        StageIdSegment::Branch(ActionBranch::PressShort),
        StageIdSegment::Index(0),
    ]);
    assert!(matches!(
        at_path(&actions, &short_path),
        Some(Action::MapToMouse {
            target: MouseTarget::LeftButton,
            ..
        })
    ));

    let long_path = StageId(vec![
        StageIdSegment::Index(1),
        StageIdSegment::Branch(ActionBranch::PressLong),
        StageIdSegment::Index(0),
    ]);
    assert!(matches!(
        at_path(&actions, &long_path),
        Some(Action::MapToMouse {
            target: MouseTarget::RightButton,
            ..
        })
    ));

    let replaced = replace_at_path(&actions, &short_path, Action::Invert)
        .expect("press short branch path must replace");
    assert!(matches!(
        at_path(&replaced, &short_path),
        Some(Action::Invert)
    ));

    let inserted = insert_at_path(&actions, &long_path, Action::Invert)
        .expect("press long branch path must insert");
    match &inserted[1] {
        Action::PressGesture { long_press, .. } => assert_eq!(long_press.len(), 2),
        _ => panic!("outer wrapper should remain PressGesture"),
    }

    let removed = remove_at_path(&actions, &long_path).expect("press long branch path must remove");
    match &removed[1] {
        Action::PressGesture { long_press, .. } => assert_eq!(long_press.len(), 0),
        _ => panic!("outer wrapper should remain PressGesture"),
    }
}

#[test]
fn generic_branch_paths_insert_replace_remove_and_reorder() {
    let actions = vec![Action::TapGesture {
        threshold_ms: 275,
        fire_single_immediately: false,
        single_tap: vec![Action::Invert],
        double_tap: vec![Action::Deadzone {
            config: DeadzoneConfig::default(),
        }],
    }];
    let single = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::TapSingle),
        StageIdSegment::Index(0),
    ]);
    let double_insert = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::TapDouble),
        StageIdSegment::Index(1),
    ]);

    let replaced = replace_at_path(
        &actions,
        &single,
        Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
    )
    .expect("generic branch path replacement succeeds");
    assert!(matches!(
        at_path(&replaced, &single),
        Some(Action::MergeAxis { .. })
    ));

    let inserted =
        insert_at_path(&replaced, &double_insert, Action::Invert).expect("branch insert succeeds");
    match &inserted[0] {
        Action::TapGesture { double_tap, .. } => assert_eq!(double_tap.len(), 2),
        _ => panic!("outer wrapper should remain TapGesture"),
    }

    let double_first = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::TapDouble),
        StageIdSegment::Index(0),
    ]);
    let moved = at_path(&inserted, &double_first)
        .cloned()
        .expect("source resolves");
    let after_remove = remove_at_path(&inserted, &double_first).expect("branch remove succeeds");
    let after_reorder =
        insert_at_path(&after_remove, &double_insert, moved).expect("branch reorder succeeds");
    match &after_reorder[0] {
        Action::TapGesture { double_tap, .. } => {
            assert!(matches!(double_tap[0], Action::Invert));
            assert!(matches!(double_tap[1], Action::Deadzone { .. }));
        }
        _ => panic!("outer wrapper should remain TapGesture"),
    }
}

fn at_path<'a>(actions: &'a [Action], path: &StageId) -> Option<&'a Action> {
    let (StageIdSegment::Index(index), parents) = path.0.split_last()? else {
        return None;
    };
    let mut list = actions;
    for pair in parents.chunks(2) {
        let [
            StageIdSegment::Index(parent),
            StageIdSegment::Branch(branch),
        ] = pair
        else {
            return None;
        };
        list = inputforge_core::action::branch_actions(list.get(*parent)?, *branch)?;
    }
    list.get(*index)
}
#[test]
fn move_dispatch_uses_post_remove_slots_and_rejects_an_ancestor_drop() {
    let actions = vec![
        Action::Invert,
        Action::Deadzone {
            config: DeadzoneConfig::default(),
        },
        Action::Invert,
    ];
    let moved = tree(
        &actions,
        Edit::Move {
            from: stage(1),
            to: stage(2),
        },
    )
    .unwrap();
    assert_eq!(
        moved,
        vec![actions[0].clone(), actions[2].clone(), actions[1].clone()]
    );
    let parent = vec![Action::Conditional {
        condition: Condition::ButtonPressed {
            input: synth_addr(),
        },
        if_true: vec![Action::Invert],
        if_false: vec![],
    }];
    assert!(
        tree(
            &parent,
            Edit::Move {
                from: stage(0),
                to: StageId(vec![
                    StageIdSegment::Index(0),
                    StageIdSegment::Branch(ActionBranch::ConditionalTrue),
                    StageIdSegment::Index(0)
                ])
            }
        )
        .is_none()
    );
}
