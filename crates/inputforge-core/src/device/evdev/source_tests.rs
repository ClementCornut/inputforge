use super::{translate::translate, *};
use crate::{
    device::InputUpdate,
    profile::controllers::{AxisBinding, DeviceBinding},
    types::*,
};
use std::{collections::HashMap, time::Instant};

fn binding() -> DeviceBinding {
    DeviceBinding {
        observed_layout: None,
        device: DeviceId("evdev:v1:test".into()),
        buttons: vec![300, 704],
        hats: vec![0],
        unavailable: vec![],
        axes: vec![AxisBinding {
            code: 5,
            minimum: 10,
            maximum: 110,
            polarity: AxisPolarity::Unipolar,
        }],
    }
}
#[test]
fn translate_snapshot_is_separate_and_frame_preserves_edges_and_coherent_hat() {
    let table = binding();
    let mut states = HashMap::new();
    let snapshot = StreamUpdate::Snapshot {
        device: table.device.clone(),
        state: NativeState {
            keys: [(300, true), (704, false)].into(),
            axes: [(5, 160), (16, 0), (17, 0)].into(),
        },
        kind: SnapshotKind::Initial,
        timestamp: Instant::now(),
    };
    let InputUpdate::Snapshot { values, .. } =
        translate(std::slice::from_ref(&table), &mut states, snapshot).unwrap()
    else {
        panic!("snapshot became edges");
    };
    assert!(
        values
            .iter()
            .any(|e| matches!(e.value, InputValue::Axis { value, .. } if (value.value() - 2.0).abs() < f64::EPSILON))
    );
    let frame = StreamUpdate::Frame {
        device: table.device.clone(),
        timestamp: Instant::now(),
        changes: vec![
            NativeChange {
                control: NativeControl::Key(704),
                value: 1,
            },
            NativeChange {
                control: NativeControl::Key(704),
                value: 0,
            },
            NativeChange {
                control: NativeControl::Abs(16),
                value: 1,
            },
            NativeChange {
                control: NativeControl::Abs(17),
                value: -1,
            },
        ],
    };
    let InputUpdate::Frame(events) = translate(&[table], &mut states, frame).unwrap() else {
        panic!("not a frame");
    };
    assert_eq!(events.len(), 3);
    assert_eq!(
        events[0].source,
        InputAddress::Bound {
            device: DeviceId("evdev:v1:test".into()),
            input: InputId::Button { index: 1 }
        }
    );
    assert_eq!(events[0].value, InputValue::Button { pressed: true });
    assert_eq!(events[1].value, InputValue::Button { pressed: false });
    assert_eq!(
        events[2].value,
        InputValue::Hat {
            direction: HatDirection::NE
        }
    );
}
#[test]
fn translate_rejects_unknown_controls_and_invalid_hats() {
    for (code, value) in [(60, 1), (16, 2)] {
        let table = binding();
        let frame = StreamUpdate::Frame {
            device: table.device.clone(),
            timestamp: Instant::now(),
            changes: vec![NativeChange {
                control: NativeControl::Abs(code),
                value,
            }],
        };
        let mut states = [(
            table.device.clone(),
            NativeState {
                keys: [(300, false), (704, false)].into(),
                axes: [(5, 10), (16, 0), (17, 0)].into(),
            },
        )]
        .into();
        translate(&[table], &mut states, frame).unwrap_err();
    }
}
