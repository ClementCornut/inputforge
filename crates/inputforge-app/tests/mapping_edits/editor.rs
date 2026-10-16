//! Editor intents share mapping dispatch, validation and queued history.
use super::support::*;
use inputforge_core::action::{Action, Mapping, OutputBehavior};
use inputforge_gui_dx::mapping_edits::{Edit, Field, StageKind};

fn inline(
    request: inputforge_gui_dx::mapping_edits::Request,
    cancel: bool,
) -> inputforge_gui_dx::mapping_edits::Inline {
    inputforge_gui_dx::mapping_edits::Inline {
        target: Some(inputforge_gui_dx::mapping_rail::Target {
            scope: inputforge_gui_dx::mapping_rail::Scope {
                profile: request.profile,
                mode: request.before.mode.clone(),
            },
            before: request.before,
        }),
        edit: request.edit,
        label: request.label,
        editor: request.editor,
        cancel,
    }
}

#[test]
fn queued_fields_preserve_disjoint_changes_and_exact_optional_names() {
    let original = Action::MapToMouse {
        target: inputforge_core::action::MouseTarget::LeftButton,
        behavior: OutputBehavior::Hold,
    };
    for name in [None, Some(String::new()), Some("Exact".into())] {
        let (controller, state, rx) = fixture(vec![original.clone()]);
        state.write().active_profile.as_mut().unwrap().set_mapping(
            &key().1,
            &key().0,
            name.clone(),
            vec![original.clone()],
        );
        let before = Mapping {
            name: name.clone(),
            ..mapping(vec![original.clone()])
        };
        for field in [
            Field::MouseTarget("right_button".into()),
            Field::Behavior(OutputBehavior::Pulse),
        ] {
            assert!(controller.dispatch(
                request(
                    &state,
                    before.clone(),
                    Edit::Field {
                        path: stage(0),
                        field
                    }
                ),
                now()
            ));
        }
        apply(&state, &rx);
        let last = apply(&state, &rx);
        assert_eq!(last.name, name);
        assert_eq!(
            last.actions,
            vec![Action::MapToMouse {
                target: inputforge_core::action::MouseTarget::RightButton,
                behavior: OutputBehavior::Pulse
            }]
        );
        assert_eq!(controller.view().stacks[&key()].undo.len(), 2);
    }
}

#[test]
fn malformed_fields_stale_stages_and_replaced_profiles_never_deliver_or_record() {
    let action = Action::TapGesture {
        threshold_ms: 250,
        fire_single_immediately: false,
        single_tap: vec![],
        double_tap: vec![],
    };
    let (controller, state, rx) = fixture(vec![action.clone()]);
    for raw in ["", "NaN", "-1", "0", "10001", "1.5"] {
        assert!(!controller.dispatch(
            request(
                &state,
                mapping(vec![action.clone()]),
                Edit::Field {
                    path: stage(0),
                    field: Field::GestureThreshold(raw.into())
                }
            ),
            now()
        ));
    }
    assert!(!controller.dispatch(
        request(
            &state,
            mapping(vec![action.clone()]),
            Edit::Field {
                path: stage(1),
                field: Field::GestureToggle(true)
            }
        ),
        now()
    ));
    let captured = request(
        &state,
        mapping(vec![action]),
        Edit::Field {
            path: stage(0),
            field: Field::GestureToggle(true),
        },
    );
    let (_, replacement_state, _) = fixture(vec![Action::Invert]);
    let replacement = replacement_state.write().active_profile.take().unwrap();
    state.write().active_profile = Some(replacement);
    assert!(!controller.dispatch(captured, now()));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert!(controller.view().stacks.is_empty());
}

#[test]
fn default_insertion_uses_app_settings_and_nested_paths() {
    use inputforge_core::action::{ActionBranch, Condition};
    use inputforge_gui_dx::mapping_edits::{StageId, StageIdSegment};
    let original = Action::Conditional {
        condition: Condition::ButtonPressed { input: key().1 },
        if_true: vec![],
        if_false: vec![],
    };
    let (controller, state, rx) = fixture(vec![original.clone()]);
    let mut before = bind(
        &state,
        inputforge_core::types::InputAddress::Bound {
            device: inputforge_core::types::DeviceId("pad".into()),
            input: inputforge_core::types::InputId::Button { index: 0 },
        },
    );
    state.write().default_double_tap_threshold_ms = 420;
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::Branch(ActionBranch::ConditionalFalse),
        StageIdSegment::Index(0),
    ]);
    assert!(controller.dispatch(
        request(
            &state,
            {
                before.actions = vec![original];
                before
            },
            Edit::Add {
                path,
                kind: StageKind::Tap
            }
        ),
        now()
    ));
    let after = apply(&state, &rx);
    let Action::Conditional { if_false, .. } = &after.actions[0] else {
        panic!("conditional expected")
    };
    assert!(matches!(
        &if_false[0],
        Action::TapGesture {
            threshold_ms: 420,
            ..
        }
    ));
}

#[test]
fn inline_cancel_and_empty_close_without_delivery_even_offline() {
    use dioxus::prelude::*;
    use inputforge_core::action::Action;
    use std::{cell::RefCell, rc::Rc};
    type Exposed = Rc<RefCell<Option<inputforge_gui_dx::mapping_edits::Bindings>>>;
    fn root() -> Element {
        let controller = use_context::<inputforge_app::mapping_edits::Controller>();
        *use_context::<Exposed>().borrow_mut() = Some(use_hook(|| controller.bindings()));
        rsx! { div {} }
    }
    let (controller, state, rx) = fixture(vec![Action::Invert]);
    let exposed = Exposed::default();
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(controller.clone());
    dom.provide_root_context(Rc::clone(&exposed));
    dom.rebuild_in_place();
    let bindings = exposed.borrow().unwrap();
    state.write().session.offline = true;
    for (name, raw, cancel) in [(None, "   ", false), (None, "Other", true)] {
        let before = Mapping {
            name: name.map(str::to_owned),
            ..mapping(vec![Action::Invert])
        };
        let intent = inline(request(&state, before, Edit::Rename(raw.into())), cancel);
        assert_eq!(bindings.inline.call(intent.clone()), Ok(false));
        assert_eq!(
            bindings
                .inline
                .call(inputforge_gui_dx::mapping_edits::Inline {
                    target: None,
                    ..intent
                }),
            Ok(false)
        );
    }
    bindings
        .inline
        .call(inline(
            request(
                &state,
                mapping(vec![Action::Invert]),
                Edit::Rename("Changed".into()),
            ),
            false,
        ))
        .unwrap_err();
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert!(controller.view().stacks.is_empty());
}

#[test]
fn gesture_toggle_uses_latest_queued_threshold_and_keeps_both_branches() {
    let original = Action::PressGesture {
        threshold_ms: 250,
        fire_long_when_threshold_crossed: false,
        short_press: vec![Action::Invert],
        long_press: vec![Action::Invert],
    };
    let (controller, state, rx) = fixture(vec![original]);
    let before = bind(
        &state,
        inputforge_core::types::InputAddress::Bound {
            device: inputforge_core::types::DeviceId("pad".into()),
            input: inputforge_core::types::InputId::Button { index: 0 },
        },
    );
    for field in [
        Field::GestureThreshold("777".into()),
        Field::GestureToggle(true),
    ] {
        assert!(controller.dispatch(
            request(
                &state,
                before.clone(),
                Edit::Field {
                    path: stage(0),
                    field
                }
            ),
            now()
        ));
    }
    apply(&state, &rx);
    let after = apply(&state, &rx);
    assert!(
        matches!(&after.actions[0], Action::PressGesture { threshold_ms: 777, fire_long_when_threshold_crossed: true, short_press, long_press } if *short_press == vec![Action::Invert] && *long_press == vec![Action::Invert])
    );
}

#[test]
fn predicate_raw_values_and_unknown_choices_cannot_mutate_the_mapping() {
    use inputforge_core::action::Condition;
    let original = Action::Conditional {
        condition: Condition::AxisInRange {
            input: key().1,
            min: -1.0,
            max: 1.0,
        },
        if_true: vec![Action::Invert],
        if_false: vec![],
    };
    let (controller, state, rx) = fixture(vec![original.clone()]);
    for field in [
        Field::ConditionMin("NaN".into()),
        Field::ConditionMax("inf".into()),
        Field::ConditionMin("2".into()),
        Field::ConditionKind("unknown".into()),
        Field::MouseTarget("unknown".into()),
    ] {
        assert!(!controller.dispatch(
            request(
                &state,
                mapping(vec![original.clone()]),
                Edit::Field {
                    path: stage(0),
                    field
                }
            ),
            now()
        ));
    }
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert!(controller.view().stacks.is_empty());
    assert!(controller.dispatch(
        request(
            &state,
            mapping(vec![original]),
            Edit::Field {
                path: stage(0),
                field: Field::ConditionMin("-0.25".into())
            }
        ),
        now()
    ));
    assert!(
        matches!(&apply(&state, &rx).actions[0], Action::Conditional { condition: Condition::AxisInRange { min: -0.25, max: 1.0, .. }, if_true, .. } if *if_true == vec![Action::Invert])
    );
}

#[test]
fn mode_strategy_and_target_reject_disabled_or_unknown_choices() {
    let original = Action::ChangeMode {
        strategy: inputforge_core::action::ModeChangeStrategy::SwitchTo {
            mode: "Default".into(),
        },
    };
    let (controller, state, rx) = fixture(vec![original.clone()]);
    for field in [Field::ModeHold(true), Field::ModeTarget("Missing".into())] {
        assert!(!controller.dispatch(
            request(
                &state,
                mapping(vec![original.clone()]),
                Edit::Field {
                    path: stage(0),
                    field
                }
            ),
            now()
        ));
    }
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}

#[test]
fn choosing_the_current_predicate_kind_preserves_its_operands_without_history() {
    let action = Action::Conditional {
        condition: inputforge_core::action::Condition::AxisInRange {
            input: key().1,
            min: -0.25,
            max: 0.75,
        },
        if_true: vec![Action::Invert],
        if_false: vec![],
    };
    let (controller, state, rx) = fixture(vec![action.clone()]);
    assert!(!controller.dispatch(
        request(
            &state,
            mapping(vec![action]),
            Edit::Field {
                path: stage(0),
                field: Field::ConditionKind("AxisInRange".into())
            }
        ),
        now()
    ));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert!(controller.view().stacks.is_empty());
}

#[test]
fn stale_mouse_behavior_control_cannot_turn_a_queued_wheel_target_into_hold() {
    let original = Action::MapToMouse {
        target: inputforge_core::action::MouseTarget::LeftButton,
        behavior: OutputBehavior::Hold,
    };
    let (controller, state, rx) = fixture(vec![original.clone()]);
    let before = mapping(vec![original]);
    assert!(controller.dispatch(
        request(
            &state,
            before.clone(),
            Edit::Field {
                path: stage(0),
                field: Field::MouseTarget("wheel_up".into())
            }
        ),
        now()
    ));
    assert!(!controller.dispatch(
        request(
            &state,
            before,
            Edit::Field {
                path: stage(0),
                field: Field::Behavior(OutputBehavior::Hold)
            }
        ),
        now()
    ));
    assert_eq!(
        apply(&state, &rx).actions,
        vec![Action::MapToMouse {
            target: inputforge_core::action::MouseTarget::WheelUp,
            behavior: OutputBehavior::Pulse
        }]
    );
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(controller.view().stacks[&key()].undo.len(), 1);
}

#[test]
fn unavailable_palette_outputs_are_revalidated_before_insertion() {
    use inputforge_core::types::{KeyCombo, PhysicalKey};
    let (controller, state, rx) = fixture(vec![]);
    for kind in [
        StageKind::Mouse,
        StageKind::Keyboard(KeyCombo {
            key: PhysicalKey::F12,
            modifiers: vec![],
        }),
    ] {
        assert!(!controller.dispatch(
            request(
                &state,
                mapping(vec![]),
                Edit::Add {
                    path: stage(0),
                    kind
                }
            ),
            now()
        ));
    }
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert!(controller.view().stacks.is_empty());
}

fn assert_queued_rename(next: &str) {
    use dioxus::prelude::*;
    use inputforge_core::action::Action;
    for editor in [false, true] {
        let (controller, state, rx) = fixture(vec![Action::Invert]);
        state.write().active_profile.as_mut().unwrap().set_mapping(
            &key().1,
            &key().0,
            Some("Old".into()),
            vec![Action::Invert],
        );
        let before = Mapping {
            name: Some("Old".into()),
            ..mapping(vec![Action::Invert])
        };
        let mut dom = VirtualDom::new(|| rsx! { div {} });
        dom.rebuild_in_place();
        let bindings = dom.in_scope(ScopeId::APP, || controller.bindings());
        let rename = |name: &str| {
            let mut r = request(&state, before.clone(), Edit::Rename(name.into()));
            r.editor = editor;
            r
        };
        assert_eq!(bindings.inline.call(inline(rename("New"), false)), Ok(true));
        assert_eq!(
            bindings.inline.call(inline(rename(next), false)),
            Ok(next == "Old")
        );
        assert_eq!(apply(&state, &rx).name.as_deref(), Some("New"));
        if next == "Old" {
            assert_eq!(apply(&state, &rx).name.as_deref(), Some("Old"));
        }
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
        assert_eq!(
            controller.view().stacks[&key()].undo.len(),
            if next == "Old" { 2 } else { 1 }
        );
    }
}

#[test]
fn inline_rename_back_to_old_uses_the_effective_queued_name() {
    assert_queued_rename("Old");
}
#[test]
fn inline_repeated_new_name_is_a_queued_noop() {
    assert_queued_rename("New");
}
