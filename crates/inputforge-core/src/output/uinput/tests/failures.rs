use super::{
    super::{default_config, device::DeviceKey},
    fixtures,
};
use std::{collections::BTreeSet, io, time::Duration};

#[test]
fn release_write_failure_invalidates_owner_even_after_every_device_is_closed() {
    let (mut output, world) = fixtures::output(vec![default_config(1)]);
    output.create().unwrap();
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::WouldBlock.into()));
    assert_eq!(
        output.release().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert!(!output.is_active());
    assert!(world.lock().unwrap().alive.is_empty());
    assert!(output.configure(vec![default_config(2)]).is_err());
    assert!(output.create().is_err());
    output.release().unwrap();
}

#[test]
fn initial_write_failure_rolls_back_all_slots_and_allows_retry() {
    let (mut output, world) = fixtures::output((1..=16).map(default_config).collect());
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::WouldBlock.into()));
    assert_eq!(
        output.create().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert!(!output.is_active());
    assert!(world.lock().unwrap().alive.is_empty());
    output.create().unwrap();
    assert_eq!(world.lock().unwrap().alive.len(), 16);
    output.release().unwrap();
    assert!(world.lock().unwrap().alive.is_empty());
}

#[test]
fn fatal_readiness_failure_rolls_back_without_wait_or_writes() {
    let (mut output, world) = fixtures::output(vec![default_config(1), default_config(2)]);
    let start = world.lock().unwrap().now;
    world.lock().unwrap().fail_ready = Some((DeviceKey::Controller(2), io::ErrorKind::InvalidData));
    assert_eq!(
        output.create().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    let script = world.lock().unwrap();
    assert_eq!(script.now, start);
    assert!(script.alive.is_empty());
    assert_eq!(script.emitted.len(), 0);
}

#[test]
fn expired_cleanup_budget_stops_writes_but_closes_every_slot() {
    let (mut output, world) = fixtures::output((1..=16).map(default_config).collect());
    output.create().unwrap();
    world.lock().unwrap().emitted.clear();
    world.lock().unwrap().write_delay = Duration::from_millis(50);
    assert_eq!(
        output.release().unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    let script = world.lock().unwrap();
    assert_eq!(script.emitted.len(), 1);
    assert!(script.alive.is_empty());
    assert_eq!(
        script
            .calls
            .iter()
            .filter(|(_, call)| *call == "close")
            .count(),
        16
    );
}

#[test]
fn slow_destroy_does_not_exhaust_later_controllers_neutral_write_budget() {
    let (mut output, world) = fixtures::output((1..=3).map(default_config).collect());
    output.create().unwrap();
    for slot in 1..=3 {
        output.set_button(slot, 1, true).unwrap();
    }
    output.flush().unwrap();
    world.lock().unwrap().emitted.clear();
    world.lock().unwrap().destroy_delay = Duration::from_millis(25);
    output.release().unwrap();
    assert!(!output.is_active());
    let script = world.lock().unwrap();
    assert!(script.alive.is_empty());
    assert_eq!(
        script
            .emitted
            .iter()
            .map(|(key, _)| *key)
            .collect::<BTreeSet<_>>(),
        [
            DeviceKey::Controller(1),
            DeviceKey::Controller(2),
            DeviceKey::Controller(3),
        ]
        .into_iter()
        .collect()
    );
    assert!(
        script
            .emitted
            .iter()
            .flat_map(|(_, events)| events)
            .all(|event| event.value() == 0)
    );
}
