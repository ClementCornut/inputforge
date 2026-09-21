use super::{
    super::{default_config, device::DeviceKey, sink::UinputSink},
    fixtures,
};
use crate::output::OutputSink;
use std::io;

#[test]
fn engine_write_failure_defers_all_cleanup_and_never_retries_failed_slot() {
    let (mut output, world) = fixtures::output(vec![default_config(1), default_config(2)]);
    output.create().unwrap();
    let mut sink = UinputSink {
        output: Some(output),
        failed_slot: None,
    };
    sink.set_axis(1, crate::types::VJoyAxis::X, 1.0).unwrap();
    world.lock().unwrap().calls.clear();
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::WouldBlock.into()));
    assert!(sink.flush().is_err());
    assert_eq!(world.lock().unwrap().alive.len(), 2);
    assert_eq!(
        world.lock().unwrap().calls,
        [(DeviceKey::Controller(1), "write")]
    );
    assert!(sink.set_button(2, 1, true).is_err());
    sink.stop().unwrap();
    let script = world.lock().unwrap();
    assert!(script.alive.is_empty());
    assert_eq!(
        script
            .calls
            .iter()
            .filter(|c| **c == (DeviceKey::Controller(1), "write"))
            .count(),
        1
    );
    drop(script);
    sink.stop().unwrap();
}

#[test]
fn neutralization_retains_identity_and_start_cannot_reconfigure_active_set() {
    let (mut output, world) = fixtures::output(vec![default_config(1)]);
    output.create().unwrap();
    let mut sink = UinputSink {
        output: Some(output),
        failed_slot: None,
    };
    sink.set_button(1, 1, true).unwrap();
    sink.neutralize().unwrap();
    assert_eq!(world.lock().unwrap().alive.len(), 1);
    assert!(sink.start(&[default_config(2)]).is_err());
    sink.stop().unwrap();
}

#[test]
fn engine_button_edges_are_submitted_even_within_one_gesture_tick() {
    let (mut output, world) = fixtures::output(vec![default_config(1)]);
    output.create().unwrap();
    let mut sink = UinputSink {
        output: Some(output),
        failed_slot: None,
    };
    world.lock().unwrap().emitted.clear();
    sink.set_button(1, 1, true).unwrap();
    sink.set_button(1, 1, false).unwrap();
    sink.flush().unwrap();
    let script = world.lock().unwrap();
    let values: Vec<_> = script
        .emitted
        .iter()
        .flat_map(|(_, events)| events)
        .filter(|e| e.event_type().0 == 1 && e.code() == 0x120)
        .map(evdev::InputEvent::value)
        .collect();
    drop(script);
    assert_eq!(values, [1, 0]);
}
