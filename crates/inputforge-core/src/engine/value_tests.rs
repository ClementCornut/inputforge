use crate::{
    action::{Action, Condition},
    pipeline::{self, InputCache},
    processing::Calibration,
    state::AppState,
    types::*,
};

#[test]
fn calibration_is_shared_by_primary_secondary_predicate_and_preview() {
    let mut state = AppState::new();
    let device = DeviceId("evdev:v1:test".into());
    let axis = InputAddress::Bound {
        device: device.clone(),
        input: InputId::Axis { index: 0 },
    };
    state.calibrations.set(
        device,
        0,
        Calibration::new(-1.0, 0.0, 0.0, 0.5, true).unwrap(),
    );
    state.input_cache.update(
        &axis,
        &InputValue::Axis {
            value: AxisValue::new(0.25),
            polarity: AxisPolarity::Bipolar,
        },
    );
    let values = crate::state::InputValues::new(&state);
    assert!((values.get_axis(&axis).0 - 0.5).abs() < f64::EPSILON);
    let actions = vec![Action::Conditional {
        condition: Condition::AxisInRange {
            input: axis.clone(),
            min: 0.49,
            max: 0.51,
        },
        if_true: vec![Action::MergeAxis {
            second_input: axis.clone(),
            operation: MergeOp::Average,
        }],
        if_false: vec![Action::Invert],
    }];
    let InputValue::Axis { value, .. } =
        pipeline::evaluate_actions_through(&actions, &state, &axis, 1)
    else {
        panic!("axis");
    };
    assert!((value.value() - 0.5).abs() < f64::EPSILON);
    assert!((state.input_cache.get_axis(&axis).0 - 0.25).abs() < f64::EPSILON);
    let mut routed = actions.clone();
    routed.push(Action::MapToVJoy {
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    });
    let mapping = crate::action::Mapping {
        input: axis.clone(),
        mode: "Default".into(),
        name: None,
        actions: routed,
    };
    let mut sink = crate::output::MockOutputSink::new();
    super::output_handler::refresh_axes_for_state(&mut state, &[mapping], "Default", &mut sink)
        .unwrap();
    assert!(
        matches!(sink.calls(), [crate::output::OutputCall::SetAxis { value, .. }] if (*value - 0.5).abs() < f64::EPSILON)
    );
    state.calibrations.set(
        axis.device().unwrap().clone(),
        0,
        Calibration::new(-1.0, 0.0, 0.0, 0.5, false).unwrap(),
    );
    assert!((crate::state::InputValues::new(&state).get_axis(&axis).0 - 0.25).abs() < f64::EPSILON);
    assert!((crate::state::InputValues::new(&state).get_axis(&axis).0 - 0.25).abs() < f64::EPSILON);
}

#[test]
fn nonfinite_calibration_is_rejected() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        Calibration::new(-1.0, 0.0, 0.0, bad, true).unwrap_err();
    }
}
