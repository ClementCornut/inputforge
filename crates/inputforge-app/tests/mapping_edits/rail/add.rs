use super::*;
use inputforge_core::{action::Action, engine::EngineCommand};
use inputforge_gui_dx::mapping_rail::{AddIntent, AddRequest, AddView, Phase};

#[test]
fn add_requires_commit_trims_names_and_tracks_queued_collisions() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let v = c.add(AddRequest {
        view: AddView::default(),
        intent: AddIntent::Start {
            scope: scope(&state),
            source: None,
        },
    });
    assert_eq!(v.phase, Phase::Capturing);
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Captured(address(1)),
    });
    assert_eq!(v.phase, Phase::Captured(address(1)));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Commit("  New  ".into()),
    });
    assert_eq!(v.phase, Phase::Resting);
    assert!(
        matches!(rx.try_recv().unwrap(), EngineCommand::SetMapping { input, name, actions, .. } if input == address(1) && name.as_deref() == Some("New") && actions.is_empty())
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::MappingEditBarrier { .. }
    ));
    let v = c.add(AddRequest {
        view: AddView::default(),
        intent: AddIntent::Start {
            scope: scope(&state),
            source: None,
        },
    });
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Captured(address(1)),
    });
    assert!(matches!(v.phase, Phase::Collision(ref m) if m.name.as_deref() == Some("New")));
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&address(1), "Default")
            .is_none()
    );
}
#[test]
fn duplicate_copies_latest_queued_actions_and_rejects_deleted_source() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let v = c.add(AddRequest {
        view: AddView::default(),
        intent: AddIntent::Start {
            scope: scope(&state),
            source: Some(target(&state)),
        },
    });
    assert!(c.dispatch(
        replace(&state, mapping(vec![Action::Invert]), 0, value(0.25)),
        now()
    ));
    let latest = apply(&state, &rx);
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Captured(address(1)),
    });
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Commit("copy".into()),
    });
    assert_eq!(v.phase, Phase::Resting);
    assert_eq!(apply(&state, &rx).actions, latest.actions);
    let v = c.add(AddRequest {
        view: AddView::default(),
        intent: AddIntent::Start {
            scope: scope(&state),
            source: Some(target(&state)),
        },
    });
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, "Default");
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Captured(address(2)),
    });
    assert_eq!(v.phase, Phase::Resting);
    assert!(v.error.is_some());
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn collision_disappearance_allows_add_but_late_collision_prevents_overwrite() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let v = c.add(AddRequest {
        view: AddView::default(),
        intent: AddIntent::Start {
            scope: scope(&state),
            source: None,
        },
    });
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Captured(key().1),
    });
    assert!(matches!(v.phase, Phase::Collision(_)));
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .remove_mapping(&key().1, "Default");
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Reconcile,
    });
    assert_eq!(v.phase, Phase::Captured(key().1));
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &key().1,
        "Default",
        Some("External".into()),
        vec![Action::Invert],
    );
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Commit("new".into()),
    });
    assert!(matches!(v.phase, Phase::Collision(ref m) if m.name.as_deref() == Some("External")));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn disconnected_commit_keeps_captured_draft_and_reports_error() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let v = c.add(AddRequest {
        view: AddView::default(),
        intent: AddIntent::Start {
            scope: scope(&state),
            source: None,
        },
    });
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Captured(address(1)),
    });
    drop(rx);
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Commit("new".into()),
    });
    assert_eq!(v.phase, Phase::Captured(address(1)));
    assert!(v.error.is_some());
    assert_eq!(c.view().stacks.len(), 0);
}
#[test]
fn profile_switch_cancels_captured_add_and_cancel_never_dispatches() {
    let (c, state, rx) = fixture(vec![Action::Invert]);
    let v = c.add(AddRequest {
        view: AddView::default(),
        intent: AddIntent::Start {
            scope: scope(&state),
            source: None,
        },
    });
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Captured(address(1)),
    });
    state.write().active_profile = fixture(vec![]).1.read().active_profile.clone();
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Commit("new".into()),
    });
    assert_eq!(v.phase, Phase::Resting);
    let v = c.add(AddRequest {
        view: v,
        intent: AddIntent::Cancel,
    });
    assert_eq!(v, AddView::default());
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
