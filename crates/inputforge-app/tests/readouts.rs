//! Regression coverage for coherent application-prepared live readouts.
use inputforge_app::bridge;
use inputforge_core::{
    action::{Action, Mapping},
    mode::Modes,
    profile::Profile,
    state::{AppState, DeviceState},
    types::{AxisPolarity, AxisValue, DeviceId, DeviceInfo, InputAddress, InputId, InputValue},
};
fn axis(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("stick".into()),
        input: InputId::Axis { index },
    }
}
fn fixture() -> AppState {
    let profile = Profile::new(
        "P".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![Mapping {
            input: axis(0),
            mode: "Default".into(),
            name: None,
            actions: vec![
                Action::Invert,
                Action::Deadzone {
                    config: inputforge_core::processing::DeadzoneConfig::default(),
                },
            ],
        }],
        vec![],
        "Default".into(),
    );
    let mut state = AppState::with_profile(profile);
    state.devices.push(DeviceState {
        info: DeviceInfo {
            id: DeviceId("stick".into()),
            name: "Stick".into(),
            axes: 1,
            buttons: 0,
            hats: 0,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar],
        },
        connected: true,
        diagnostics: inputforge_core::types::DeviceDiagnostics::default(),
    });
    state.input_cache.update(
        &axis(0),
        &InputValue::Axis {
            value: AxisValue::new(0.6),
            polarity: AxisPolarity::Bipolar,
        },
    );
    state
}
#[test]
fn prepared_instrument_value_matches_core_and_rejects_disconnected_input() {
    let mut state = fixture();
    let key = ("Default".into(), axis(0));
    let frame = bridge::prepare(&state, Some(&key));
    let readout = frame.live.readout.as_ref().unwrap();
    let expected = inputforge_core::pipeline::evaluate_actions_through(
        &state.active_profile.as_ref().unwrap().mappings()[0].actions,
        &state,
        &axis(0),
        1,
    );
    let InputValue::Axis { value, .. } = expected else {
        panic!("axis fixture")
    };
    assert_eq!(readout.instruments[1], Some(value.value()));
    assert_eq!(frame.live.input_readouts[&axis(0)].intensity, 0.6);
    state.devices[0].connected = false;
    let frame = bridge::prepare(&state, Some(&key));
    assert_eq!(frame.live.readout.unwrap().instruments[1], None);
}
#[test]
fn replacing_profile_or_actions_replaces_readout_identity_and_missing_selection_has_no_model() {
    let mut state = fixture();
    let key = ("Default".into(), axis(0));
    let first = bridge::prepare(&state, Some(&key)).live.readout.unwrap();
    state.active_profile = fixture().active_profile;
    let replacement = bridge::prepare(&state, Some(&key)).live.readout.unwrap();
    assert_ne!(first.key, replacement.key);
    state.active_profile.as_mut().unwrap().set_mapping(
        &axis(0),
        "Default",
        None,
        vec![Action::Invert],
    );
    let changed = bridge::prepare(&state, Some(&key)).live.readout.unwrap();
    assert_ne!(changed.key, replacement.key);
    assert!(bridge::prepare(&state, None).live.readout.is_none());
}
#[test]
fn app_expansion_reset_rejects_a_toggle_captured_before_target_replacement() {
    use dioxus::prelude::*;
    let mut dom = VirtualDom::new(|| rsx! {});
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let mut state = fixture();
        let key = ("Default".into(), axis(0));
        let first = bridge::prepare(&state, Some(&key))
            .live
            .readout
            .unwrap()
            .key;
        state.active_profile = fixture().active_profile;
        let replacement = bridge::prepare(&state, Some(&key))
            .live
            .readout
            .unwrap()
            .key;
        let (tx, _rx) = std::sync::mpsc::channel();
        let controller = inputforge_app::mapping_edits::Controller::new(
            std::sync::Arc::new(parking_lot::RwLock::new(state)),
            tx,
        );
        let expand = controller.bindings().readouts.expand;
        let current = expand.call(inputforge_gui_dx::readouts::Expansion {
            state: inputforge_gui_dx::readouts::ExpandState::default(),
            key: first.clone(),
            outputs: 2,
            toggle: None,
        });
        let current = expand.call(inputforge_gui_dx::readouts::Expansion {
            state: current,
            key: first.clone(),
            outputs: 2,
            toggle: Some(0),
        });
        assert!(current.is_expanded(0));
        let reset = expand.call(inputforge_gui_dx::readouts::Expansion {
            state: current,
            key: replacement.clone(),
            outputs: 2,
            toggle: None,
        });
        assert!(!reset.is_expanded(0));
        let changed = expand.call(inputforge_gui_dx::readouts::Expansion {
            state: reset,
            key: replacement,
            outputs: 2,
            toggle: Some(1),
        });
        assert!(changed.is_expanded(1));
        let rejected = expand.call(inputforge_gui_dx::readouts::Expansion {
            state: changed.clone(),
            key: first,
            outputs: 2,
            toggle: Some(0),
        });
        assert_eq!(rejected, changed);
    });
}
#[test]
fn missing_button_primary_keeps_its_binary_readout_and_no_tracking_dot() {
    use inputforge_gui_dx::readouts::ReadoutDisplay;
    let mut state = fixture();
    let button = InputAddress::Bound {
        device: DeviceId("absent".into()),
        input: InputId::Button { index: 7 },
    };
    state.active_profile.as_mut().unwrap().set_mapping(
        &button,
        "Default",
        None,
        vec![Action::Invert],
    );
    let frame = bridge::prepare(&state, Some(&("Default".into(), button.clone())));
    assert_eq!(
        frame
            .live
            .input_readouts
            .get(&button)
            .map(|value| value.display),
        Some(ReadoutDisplay::Button { pressed: false })
    );
    assert_eq!(frame.live.readout.unwrap().instruments, [None]);
}
#[test]
fn nested_merge_preview_matches_core_path_evaluation_and_owns_its_exact_branch() {
    use inputforge_core::{
        action::{ActionBranch, Condition, OutputBehavior},
        pipeline::{
            ActionPathSegment, BranchStep, OutputDestination, evaluate_actions_through_path,
        },
        types::{MergeOp, OutputAddress, OutputId, VJoyAxis},
    };
    use inputforge_gui_dx::readouts::ChainStep;
    let mut state = fixture();
    let key = ("Default".into(), axis(0));
    let watched = InputAddress::Bound {
        device: DeviceId("stick".into()),
        input: InputId::Button { index: 0 },
    };
    state
        .input_cache
        .update(&watched, &InputValue::Button { pressed: true });
    let out = OutputAddress {
        device: 2,
        output: OutputId::Axis { id: VJoyAxis::Z },
    };
    let actions = vec![
        Action::MergeAxis {
            second_input: axis(0),
            operation: MergeOp::Average,
        },
        Action::Conditional {
            condition: Condition::ButtonPressed { input: watched },
            if_true: vec![
                Action::Invert,
                Action::MergeAxis {
                    second_input: axis(0),
                    operation: MergeOp::Average,
                },
                Action::MapToVJoy {
                    output: out.clone(),
                },
            ],
            if_false: vec![],
        },
    ];
    state
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&axis(0), "Default", None, actions.clone());
    let readout = bridge::prepare(&state, Some(&key)).live.readout.unwrap();
    let InputValue::Axis { value, .. } = evaluate_actions_through_path(
        &actions,
        &state,
        &axis(0),
        &[BranchStep::Branch {
            index: 1,
            branch: ActionBranch::ConditionalTrue,
        }],
        2,
    ) else {
        panic!("axis fixture")
    };
    let ChainStep::Merge { display_value, .. } = readout.model.outputs[0].chain.last().unwrap()
    else {
        panic!("merge row")
    };
    assert_eq!(*display_value, 0.0);
    assert_eq!(*display_value, value.value());
    let owner = &readout.model.outputs[0].owner;
    assert_eq!(
        owner.action_path,
        vec![
            ActionPathSegment::Index(1),
            ActionPathSegment::Branch(ActionBranch::ConditionalTrue),
            ActionPathSegment::Index(2)
        ]
    );
    assert_eq!(owner.destination, OutputDestination::VJoy(out));
    assert_eq!(owner.behavior, OutputBehavior::Hold);
}
#[test]
fn sparse_native_axis_codes_keep_logical_values_and_unipolar_display() {
    use inputforge_core::profile::controllers::{AxisBinding, DeviceBinding};
    use inputforge_gui_dx::readouts::{AxisDisplay, ReadoutDisplay};
    let mut state = fixture();
    state.devices[0].info.axes = 2;
    state.devices[0]
        .info
        .axis_polarities
        .push(AxisPolarity::Unipolar);
    state.session.bindings = vec![DeviceBinding {
        device: DeviceId("stick".into()),
        observed_layout: None,
        unavailable: vec![],
        buttons: vec![],
        hats: vec![],
        axes: vec![
            AxisBinding {
                code: 9,
                minimum: -100,
                maximum: 100,
                polarity: AxisPolarity::Bipolar,
            },
            AxisBinding {
                code: 73,
                minimum: -100,
                maximum: 100,
                polarity: AxisPolarity::Unipolar,
            },
        ],
    }];
    state.input_cache.update(
        &axis(1),
        &InputValue::Axis {
            value: AxisValue::new(-1.0),
            polarity: AxisPolarity::Unipolar,
        },
    );
    let frame = bridge::prepare(&state, Some(&("Default".into(), axis(0))));
    assert_eq!(frame.live.input_readouts[&axis(0)].intensity, 0.6);
    assert_eq!(
        frame.live.input_readouts[&axis(1)].display,
        ReadoutDisplay::Axis(AxisDisplay {
            value: 0.0,
            polarity: AxisPolarity::Unipolar
        })
    );
    assert_eq!(frame.live.input_readouts[&axis(1)].intensity, 0.0);
}
