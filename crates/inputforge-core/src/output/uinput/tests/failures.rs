use super::{super::default_config, fixtures};
use std::{io, time::Duration};

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
    world.lock().unwrap().fail_ready = Some((2, io::ErrorKind::InvalidData));
    assert_eq!(
        output.create().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    let script = world.lock().unwrap();
    assert_eq!(script.now, start);
    assert!(script.alive.is_empty());
    assert!(script.emitted.is_empty());
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
