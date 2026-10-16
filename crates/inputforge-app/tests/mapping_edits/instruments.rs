use super::support::*;
use dioxus::prelude::*;
use inputforge_core::action::Action;
use inputforge_gui_dx::{
    instruments::{Event, Observation, Open, View},
    mapping_edits::Bindings,
};
use std::{cell::RefCell, rc::Rc};

fn bindings(controller: inputforge_app::mapping_edits::Controller) -> (VirtualDom, Bindings) {
    fn root() -> Element {
        let shared = use_context::<Rc<RefCell<Option<Bindings>>>>();
        let controller = use_context::<inputforge_app::mapping_edits::Controller>();
        *shared.borrow_mut() = Some(use_hook(|| controller.bindings()));
        rsx! { div {} }
    }
    let shared = Rc::new(RefCell::new(None));
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(controller);
    dom.provide_root_context(Rc::clone(&shared));
    dom.rebuild_in_place();
    let b = shared.borrow().unwrap();
    (dom, b)
}
fn open(
    b: &Bindings,
    state: &std::sync::Arc<parking_lot::RwLock<inputforge_core::state::AppState>>,
    action: Action,
) -> View {
    let before = mapping(vec![action.clone()]);
    b.instruments.open.call(Open {
        target: Some(inputforge_gui_dx::mapping_rail::Target {
            scope: inputforge_gui_dx::mapping_rail::Scope {
                profile: state.read().active_profile.as_ref().map(|p| p.id().clone()),
                mode: before.mode.clone(),
            },
            before,
        }),
        path: stage(0),
        seed: action,
    })
}
fn pointer(kind: &str, x: f64, y: f64, hit: Option<usize>) -> Event {
    Event::Pointer {
        kind: kind.into(),
        observation: Observation {
            position: Some((x, y)),
            hit,
            centers: false,
            split_dx: 0.0,
        },
    }
}
#[test]
fn instrument_bindings_drag_commit_once_and_cancel_without_history() {
    let action = value(1.0);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action);
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    call(pointer("down", 1.0, 1.0, Some(1)));
    call(pointer("up", 1.0, 1.0, Some(1)));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    call(pointer("down", 1.0, 1.0, Some(1)));
    let draft = call(pointer("move", 1.0, 0.5, None));
    assert_eq!(draft.value, value(0.5));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    call(pointer("up", 1.0, 0.5, None));
    assert_eq!(apply(&state, &rx).actions, vec![value(0.5)]);
    call(pointer("up", 1.0, 0.5, None));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(controller.view().stacks[&key()].undo.len(), 1);
    call(pointer("down", 1.0, 0.5, Some(1)));
    call(pointer("move", 1.0, 0.2, None));
    assert_eq!(call(Event::Cancel).value, value(0.5));
    call(pointer("down", 1.0, 0.5, Some(1)));
    call(pointer("move", 1.0, 0.2, None));
    assert_eq!(call(pointer("cancel", 1.0, 0.2, None)).value, value(0.5));
    call(pointer("up", 1.0, 0.2, None));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn instrument_failed_drag_keeps_draft_and_accessible_error() {
    let action = value(1.0);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action);
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    call(pointer("down", 1.0, 1.0, Some(1)));
    call(pointer("move", 1.0, 0.4, None));
    drop(rx);
    let failed = call(pointer("up", 1.0, 0.4, None));
    assert_eq!(failed.value, value(0.4));
    assert!(failed.error.is_some());
    assert!(controller.view().stacks.is_empty());
    assert_eq!(call(Event::Refresh).value, value(0.4));
    assert_eq!(call(Event::Cancel).value, value(1.0));
}
#[test]
fn instrument_disposal_rejects_late_release_and_fresh_mount_is_independent() {
    let action = value(1.0);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller);
    let old = open(&b, &state, action.clone());
    b.instruments
        .event
        .call((old.id, pointer("down", 1.0, 1.0, Some(1))));
    b.instruments
        .event
        .call((old.id, pointer("move", 1.0, 0.4, None)));
    b.instruments.close.call(old.id);
    let fresh = open(&b, &state, action);
    assert_ne!(old.id, fresh.id);
    assert!(
        b.instruments
            .event
            .call((old.id, pointer("up", 1.0, 0.4, None)))
            .is_none()
    );
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(fresh.value, value(1.0));
}
fn key_event(key: &str, shift: bool) -> Event {
    Event::Key {
        key: key.into(),
        shift,
    }
}
#[test]
fn instrument_queued_nudges_use_effective_mapping_and_group_until_blur() {
    let action = value(0.5);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action);
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    call(key_event("End", false));
    call(key_event("ArrowUp", false));
    let second = call(key_event("ArrowUp", false));
    assert_eq!(second.value, value(0.52));
    assert_eq!(controller.view().stacks[&key()].undo.len(), 1);
    call(Event::Blur);
    call(key_event("ArrowDown", true));
    assert_eq!(controller.view().stacks[&key()].undo.len(), 2);
    assert_eq!(apply(&state, &rx).actions, vec![value(0.51)]);
    assert_eq!(apply(&state, &rx).actions, vec![value(0.52)]);
    let final_mapping = apply(&state, &rx);
    let Action::ResponseCurve {
        curve: inputforge_core::processing::curves::ResponseCurve::PiecewiseLinear { points, .. },
    } = &final_mapping.actions[0]
    else {
        panic!("expected curve");
    };
    assert!((points[1].1 - 0.42).abs() < 1e-12);
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn instrument_external_undo_during_drag_cancels_without_late_commit() {
    let action = value(0.5);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action);
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    call(key_event("End", false));
    call(key_event("ArrowUp", false));
    apply(&state, &rx);
    call(pointer("down", 1.0, 0.51, Some(1)));
    call(pointer("move", 1.0, 0.2, None));
    assert!(controller.undo(&key()));
    let after = call(pointer("up", 1.0, 0.2, None));
    assert_eq!(after.value, value(0.5));
    assert!(after.dragging.is_none());
    assert!(after.error.is_some());
    apply(&state, &rx);
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn instrument_replaced_profile_or_deleted_stage_rejects_release() {
    for profile in [false, true] {
        let action = value(0.5);
        let (controller, state, rx) = fixture(vec![action.clone()]);
        let (_dom, b) = bindings(controller);
        let v = open(&b, &state, action);
        let call = |event| b.instruments.event.call((v.id, event)).unwrap();
        call(pointer("down", 1.0, 0.5, Some(1)));
        call(pointer("move", 1.0, 0.2, None));
        if profile {
            let replacement = fixture(vec![value(0.5)]).1.read().active_profile.clone();
            state.write().active_profile = replacement;
        } else {
            state.write().active_profile.as_mut().unwrap().set_mapping(
                &key().1,
                &key().0,
                None,
                vec![Action::Invert],
            );
        }
        let v = call(pointer("up", 1.0, 0.2, None));
        assert!(v.error.is_some());
        assert!(v.dragging.is_none());
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    }
}
#[test]
fn instrument_structural_remove_and_same_value_reinsert_cancels_old_drag() {
    let action = value(0.5);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action.clone());
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    call(pointer("down", 1.0, 0.5, Some(1)));
    call(pointer("move", 1.0, 0.2, None));
    assert!(controller.dispatch(
        request(
            &state,
            mapping(vec![action.clone()]),
            inputforge_gui_dx::mapping_edits::Edit::Remove(stage(0))
        ),
        now()
    ));
    let empty = apply(&state, &rx);
    assert!(controller.dispatch(
        request(
            &state,
            empty,
            inputforge_gui_dx::mapping_edits::Edit::Insert {
                path: stage(0),
                action
            }
        ),
        now()
    ));
    apply(&state, &rx);
    assert!(call(pointer("up", 1.0, 0.2, None)).error.is_some());
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn instrument_deadzone_numeric_validation_and_queued_cross_field_edits() {
    use inputforge_core::processing::deadzone::DeadzoneConfig;
    let action = Action::Deadzone {
        config: DeadzoneConfig::default(),
    };
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action.clone());
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    for value in [f64::NAN, f64::INFINITY, 0.1] {
        let rejected = call(Event::Number { field: 0, value });
        assert!(rejected.error.is_some());
        assert_eq!(rejected.value, action);
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    }
    call(Event::Number {
        field: 0,
        value: -0.8,
    });
    call(Event::Number {
        field: 1,
        value: -0.1,
    });
    let first = apply(&state, &rx);
    let second = apply(&state, &rx);
    assert_eq!(
        first.actions,
        vec![Action::Deadzone {
            config: DeadzoneConfig::new(-0.8, 0.0, 0.0, 1.0).unwrap()
        }]
    );
    assert_eq!(
        second.actions,
        vec![Action::Deadzone {
            config: DeadzoneConfig::new(-0.8, -0.1, 0.0, 1.0).unwrap()
        }]
    );
    call(Event::Number {
        field: 1,
        value: -0.1,
    });
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(controller.view().stacks[&key()].undo.len(), 2);
}
#[test]
fn instrument_deadzone_stacked_center_drag_and_escape_choose_no_late_commit() {
    use inputforge_core::processing::deadzone::DeadzoneConfig;
    let action = Action::Deadzone {
        config: DeadzoneConfig::default(),
    };
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action.clone());
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    let down = Event::Pointer {
        kind: "down".into(),
        observation: Observation {
            position: Some((0.0, 0.0)),
            hit: Some(1),
            centers: true,
            split_dx: 0.0,
        },
    };
    assert!(call(down.clone()).split);
    call(Event::Pointer {
        kind: "move".into(),
        observation: Observation {
            position: Some((0.1, 0.0)),
            hit: None,
            centers: true,
            split_dx: 12.0,
        },
    });
    assert_eq!(call(key_event("Escape", false)).value, action);
    call(pointer("up", 0.1, 0.0, None));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    call(down);
    call(Event::Pointer {
        kind: "move".into(),
        observation: Observation {
            position: Some((0.1, 0.0)),
            hit: None,
            centers: true,
            split_dx: 12.0,
        },
    });
    call(pointer("up", 0.1, 0.0, None));
    assert_eq!(
        apply(&state, &rx).actions,
        vec![Action::Deadzone {
            config: DeadzoneConfig::new(-1.0, 0.0, 0.1, 1.0).unwrap()
        }]
    );
    assert_eq!(controller.view().stacks[&key()].undo.len(), 1);
}
#[test]
fn instrument_nonfinite_curve_release_reverts_and_exposes_error() {
    let action = value(1.0);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller);
    let v = open(&b, &state, action.clone());
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    call(pointer("down", 1.0, 1.0, Some(1)));
    call(pointer("move", 1.0, f64::NAN, None));
    let result = call(pointer("up", 1.0, 0.1, None));
    assert_eq!(result.value, action);
    assert!(result.error.is_some());
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}
#[test]
fn instrument_tightly_spaced_valid_curve_handles_do_not_panic_or_cross_neighbors() {
    use inputforge_core::processing::curves::ResponseCurve;
    let action = Action::ResponseCurve {
        curve: ResponseCurve::piecewise_linear(
            vec![
                (-1.0, -1.0),
                (-0.0001, 0.0),
                (0.0, 0.0),
                (0.0001, 0.0),
                (1.0, 1.0),
            ],
            false,
        )
        .unwrap(),
    };
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller);
    let v = open(&b, &state, action);
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    call(pointer("down", 0.0, 0.0, Some(2)));
    call(pointer("move", 0.4, 0.4, None));
    call(pointer("up", 0.4, 0.4, None));
    let mapping = apply(&state, &rx);
    let Action::ResponseCurve {
        curve: ResponseCurve::PiecewiseLinear { points, .. },
    } = &mapping.actions[0]
    else {
        panic!("expected curve");
    };
    assert!(points[1].0 < points[2].0 && points[2].0 < points[3].0);
}
#[test]
fn instrument_stale_toolbar_click_must_not_edit_replacement_before_refresh() {
    let action = value(0.5);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller);
    let v = open(&b, &state, action);
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &key().1,
        &key().0,
        None,
        vec![value(0.3)],
    );
    let rejected = b.instruments.event.call((v.id, Event::Reset)).unwrap();
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(rejected.value, value(0.3));
    assert!(rejected.error.is_some());
}
#[test]
fn instrument_deadzone_vertical_key_splits_horizontal_nudge_group() {
    use inputforge_core::processing::deadzone::DeadzoneConfig;
    let action = Action::Deadzone {
        config: DeadzoneConfig::default(),
    };
    let (controller, state, _rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action);
    let call = |event| b.instruments.event.call((v.id, event));
    call(key_event("Home", false));
    call(key_event("ArrowRight", false));
    call(key_event("ArrowUp", false));
    call(key_event("ArrowRight", false));
    assert_eq!(controller.view().stacks[&key()].undo.len(), 2);
}
#[test]
fn instrument_topology_changes_clamp_focused_anchor_before_next_nudge() {
    use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};
    for operation in ["type", "reset", "remove"] {
        let initial = if operation == "type" {
            Action::ResponseCurve {
                curve: ResponseCurve::cubic_bezier(
                    vec![BezierSegment {
                        start: (-1.0, -1.0),
                        control1: (-0.3, -0.3),
                        control2: (0.3, 0.3),
                        end: (1.0, 0.5),
                    }],
                    false,
                )
                .unwrap(),
            }
        } else {
            Action::ResponseCurve {
                curve: ResponseCurve::piecewise_linear(
                    vec![
                        (-1.0, -1.0),
                        (-0.5, -0.5),
                        (0.0, 0.0),
                        (0.5, 0.5),
                        (1.0, 0.5),
                    ],
                    false,
                )
                .unwrap(),
            }
        };
        let (controller, state, rx) = fixture(vec![initial.clone()]);
        let (_dom, b) = bindings(controller.clone());
        let v = open(&b, &state, initial);
        let call = |event| b.instruments.event.call((v.id, event)).unwrap();
        call(key_event("End", false));
        let changed = call(match operation {
            "type" => Event::Type("linear".into()),
            "reset" => Event::Reset,
            _ => pointer("ctx", -0.5, -0.5, Some(1)),
        });
        assert_eq!(
            changed.focused,
            Some(if operation == "remove" { 3 } else { 2 }),
            "{operation}"
        );
        apply(&state, &rx);
        call(key_event("ArrowDown", false));
        let after = apply(&state, &rx);
        let Action::ResponseCurve {
            curve: ResponseCurve::PiecewiseLinear { points, .. },
        } = &after.actions[0]
        else {
            panic!("expected linear curve");
        };
        assert!(
            (points.last().unwrap().1 - if operation == "remove" { 0.49 } else { 0.99 }).abs()
                < 1e-12,
            "{operation}"
        );
        assert_eq!(controller.view().stacks[&key()].undo.len(), 2);
    }
}
#[test]
fn instrument_idle_hover_and_unhandled_keys_preserve_the_existing_nudge_burst() {
    for observation in [pointer("move", 0.0, 0.0, None), key_event("F12", false)] {
        let action = value(0.5);
        let (controller, state, _rx) = fixture(vec![action.clone()]);
        let (_dom, b) = bindings(controller.clone());
        let v = open(&b, &state, action);
        let call = |event| b.instruments.event.call((v.id, event));
        call(key_event("End", false));
        call(key_event("ArrowUp", false));
        call(observation);
        call(key_event("ArrowUp", false));
        assert_eq!(controller.view().stacks[&key()].undo.len(), 1);
    }
}
#[test]
fn instrument_cancelled_failed_topology_draft_clamps_focus_before_nudge() {
    let action = value(0.5);
    let (controller, state, rx) = fixture(vec![action.clone()]);
    let (_dom, b) = bindings(controller.clone());
    let v = open(&b, &state, action.clone());
    let call = |event| b.instruments.event.call((v.id, event)).unwrap();
    drop(rx);
    let failed = call(Event::Type("bezier".into()));
    assert!(failed.error.is_some());
    assert_eq!(call(key_event("End", false)).focused, Some(3));
    let restored = call(key_event("Escape", false));
    assert_eq!(restored.value, action);
    assert_eq!(restored.focused, Some(1));
    let nudged = call(key_event("ArrowUp", false));
    assert_eq!(nudged.value, value(0.51));
    assert!(nudged.error.is_some());
    assert!(controller.view().stacks.is_empty());
}
