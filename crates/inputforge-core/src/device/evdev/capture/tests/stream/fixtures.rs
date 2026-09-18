use super::super::fixtures::{self, Fake, id};
use crate::device::evdev::{AxisInfo, Capture, NativeState, StreamUpdate};
use evdev::InputEvent;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub(crate) struct StreamIo {
    pub reads: VecDeque<Result<Vec<InputEvent>, i32>>,
    pub state: NativeState,
    pub during_snapshot: Vec<InputEvent>,
    pub reads_count: usize,
    pub snapshots: usize,
}

pub(super) fn event(kind: u16, code: u16, value: i32) -> InputEvent {
    InputEvent::new(kind, code, value)
}
pub(super) fn report() -> InputEvent {
    event(0, 0, 0)
}
pub(super) fn dropped() -> InputEvent {
    event(0, 3, 0)
}

pub(super) fn setup(names: &[&str]) -> (Fake, Capture) {
    let fake = fixtures::world();
    for dev in &mut fake.borrow_mut().devices {
        dev.metadata.keys.extend([0x120, 0x2c0]);
        dev.metadata.abs_axes.extend([0, 0x10, 0x11]);
        dev.axes = dev
            .metadata
            .abs_axes
            .iter()
            .map(|&code| AxisInfo {
                code,
                minimum: -1,
                maximum: 1,
                fuzz: 0,
                flat: 0,
                resolution: 0,
            })
            .collect();
    }
    for name in ["a", "b", "c"] {
        let state = NativeState {
            keys: [(0x120, false), (0x2c0, true)].into(),
            axes: [(0, 123), (0x10, 0), (0x11, -1)].into(),
        };
        fake.borrow_mut().streams.insert(
            name.into(),
            StreamIo {
                state,
                ..StreamIo::default()
            },
        );
    }
    let mut capture = fixtures::capture(&fake);
    capture
        .acquire(names.iter().map(|name| id(name)).collect::<Vec<_>>())
        .unwrap();
    (fake, capture)
}

pub(super) fn ready(fake: &Fake, capture: &mut Capture) {
    let mut updates = Vec::new();
    for _ in 0..4 {
        if capture.poll_stream(&mut updates).unwrap().ready {
            return;
        }
    }
    panic!("fixture did not initialize: {:?}", fake.borrow().calls);
}

pub(super) fn queue(fake: &Fake, name: &str, events: Vec<InputEvent>) {
    fake.borrow_mut()
        .streams
        .get_mut(name)
        .unwrap()
        .reads
        .push_back(Ok(events));
}

pub(super) fn frames(updates: &[StreamUpdate]) -> usize {
    updates
        .iter()
        .filter(|u| matches!(u, StreamUpdate::Frame { .. }))
        .count()
}
