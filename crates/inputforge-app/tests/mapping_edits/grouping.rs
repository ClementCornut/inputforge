use super::support::*;
use inputforge_core::action::Action;
use inputforge_gui_dx::mapping_edits::Group;
use std::time::Duration;
#[test]
fn burst_undo_and_redo_restore_initial_and_final_values() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let start = now();
    let mut before = mapping(vec![Action::Invert]);
    for (ms, max) in [(0, 0.25), (250, 0.5), (500, 0.75)] {
        let mut r = replace(&state, before, 0, value(max));
        r.group = Some(Group {
            path: stage(0),
            key: "right".into(),
        });
        assert!(c.dispatch(r, start + Duration::from_millis(ms)));
        before = apply(&state, &rx);
    }
    assert_eq!(c.view().stacks[&key()].undo.len(), 1);
    assert!(c.undo(&key()));
    assert_eq!(apply(&state, &rx).actions, vec![Action::Invert]);
    assert!(c.redo(&key()));
    assert_eq!(apply(&state, &rx).actions, vec![value(0.75)]);
}
#[test]
fn timeout_other_stage_intervening_edit_and_focus_reset_split_groups() {
    for split in 0..4 {
        let (c, state, rx) = fixture(vec![Action::Invert, Action::Invert]);
        let start = now();
        let mut r = replace(
            &state,
            mapping(vec![Action::Invert, Action::Invert]),
            0,
            value(0.25),
        );
        r.group = Some(Group {
            path: stage(0),
            key: "right".into(),
        });
        assert!(c.dispatch(r, start));
        let mut before = apply(&state, &rx);
        if split == 2 {
            assert!(c.dispatch(
                request(
                    &state,
                    before,
                    inputforge_gui_dx::mapping_edits::Edit::Rename("name".into())
                ),
                start
            ));
            before = apply(&state, &rx);
        }
        if split == 3 {
            c.end_group();
        }
        let index = usize::from(split == 1);
        let mut r = replace(&state, before, index, value(0.5));
        r.group = Some(Group {
            path: stage(index),
            key: "right".into(),
        });
        assert!(c.dispatch(
            r,
            start + Duration::from_millis(if split == 0 { 251 } else { 100 })
        ));
        assert_eq!(
            c.view().stacks[&key()].undo.len(),
            if split == 2 { 3 } else { 2 }
        );
    }
}
