use super::{super::default_config, fixtures};
use std::{io, time::Duration};

#[test]
fn multi_device_creation_initializes_neutral_and_rejects_active_reconfiguration() {
    let (mut output, world) = fixtures::output(vec![default_config(2), default_config(1)]);
    output.create().unwrap();
    assert!(output.is_active());
    assert!(output.create().is_err());
    assert!(output.configure(vec![default_config(3)]).is_err());
    assert_eq!(
        world
            .lock()
            .unwrap()
            .alive
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(
        world
            .lock()
            .unwrap()
            .emitted
            .iter()
            .flat_map(|(_, events)| events)
            .all(|event| event.value() == 0)
    );
    let count = world.lock().unwrap().emitted.len();
    output.flush().unwrap();
    assert_eq!(world.lock().unwrap().emitted.len(), count);
    output.set_button(2, 1, true).unwrap();
    output.flush().unwrap();
    assert_eq!(world.lock().unwrap().emitted.last().unwrap().0, 2);
    output.release().unwrap();
    output.release().unwrap();
    assert!(!output.is_active());
    assert!(world.lock().unwrap().alive.is_empty());
    output.configure(vec![default_config(3)]).unwrap();
    output.create().unwrap();
    drop(output);
    assert!(world.lock().unwrap().alive.is_empty());
}

#[test]
fn failed_creation_rolls_back_and_retains_primary_and_destroy_errors() {
    let (mut output, world) = fixtures::output((1..=3).map(default_config).collect());
    world.lock().unwrap().fail_create = Some(3);
    world.lock().unwrap().fail_destroy.extend([1, 2]);
    let error = output.create().unwrap_err();
    assert_eq!(error.slot(), Some(3));
    assert_eq!(error.raw_os_error(), Some(13));
    assert!(!error.cleanup_failures().is_empty());
    assert!(error.to_string().contains("slot 1"));
    assert!(error.to_string().contains("slot 2"));
    assert!(world.lock().unwrap().alive.is_empty());
    assert!(!output.is_active());
    world.lock().unwrap().fail_create = None;
    world.lock().unwrap().fail_destroy.clear();
    output.create().unwrap();
    output.release().unwrap();
}

#[test]
fn readiness_timeout_closes_every_created_device_with_a_shared_deadline() {
    let (mut output, world) = fixtures::output(vec![default_config(1), default_config(2)]);
    let start = world.lock().unwrap().now;
    world.lock().unwrap().fail_ready = Some((2, io::ErrorKind::PermissionDenied));
    let error = output.create().unwrap_err();
    assert_eq!(error.slot(), Some(2));
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    let script = world.lock().unwrap();
    assert!(script.now - start <= Duration::from_secs(2));
    assert!(script.alive.is_empty());
    assert!(script.emitted.is_empty());
}

#[test]
fn partially_emitted_packet_failure_invalidates_owner_without_replaying_it() {
    let (mut output, world) = fixtures::output(vec![default_config(1), default_config(2)]);
    output.create().unwrap();
    world.lock().unwrap().emitted.clear();
    output.set_button(1, 1, true).unwrap();
    world.lock().unwrap().writes.extend([
        Ok(size_of::<evdev::InputEvent>()),
        Err(io::Error::from_raw_os_error(5)),
    ]);
    let error = output.flush().unwrap_err();
    assert_eq!(error.raw_os_error(), Some(5));
    assert!(!output.is_active());
    assert!(output.create().is_err());
    assert!(world.lock().unwrap().alive.is_empty());
    assert_eq!(
        world
            .lock()
            .unwrap()
            .emitted
            .iter()
            .filter(|(slot, _)| *slot == 1)
            .count(),
        1
    );
    output.release().unwrap();
}

#[test]
fn release_discards_pending_press_and_closes_all_despite_multiple_errors() {
    let (mut output, world) = fixtures::output(vec![default_config(1), default_config(2)]);
    output.create().unwrap();
    world.lock().unwrap().emitted.clear();
    output.set_button(1, 1, true).unwrap();
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::WouldBlock.into()));
    world.lock().unwrap().fail_destroy.extend([1, 2]);
    let error = output.release().unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert!(error.to_string().contains("slot 1"));
    assert!(error.to_string().contains("slot 2"));
    assert!(world.lock().unwrap().alive.is_empty());
    assert!(
        world
            .lock()
            .unwrap()
            .emitted
            .iter()
            .flat_map(|(_, events)| events)
            .all(|event| event.value() == 0)
    );
    let calls = world.lock().unwrap().calls.len();
    output.release().unwrap();
    drop(output);
    assert_eq!(world.lock().unwrap().calls.len(), calls);
}

#[test]
fn drop_destroys_without_flushing_pending_values() {
    let (mut output, world) = fixtures::output(vec![default_config(1)]);
    output.create().unwrap();
    world.lock().unwrap().emitted.clear();
    output.set_button(1, 1, true).unwrap();
    drop(output);
    assert!(world.lock().unwrap().emitted.is_empty());
    assert!(world.lock().unwrap().alive.is_empty());
}

#[test]
fn reset_releases_committed_state_and_preserves_the_active_set() {
    let (mut output, world) = fixtures::output(vec![default_config(1)]);
    output.create().unwrap();
    output.set_button(1, 1, true).unwrap();
    output.flush().unwrap();
    world.lock().unwrap().emitted.clear();
    output.reset().unwrap();
    let script = world.lock().unwrap();
    assert_eq!(script.alive.len(), 1);
    assert_eq!(
        script.emitted[0]
            .1
            .iter()
            .map(|e| (e.event_type().0, e.code(), e.value()))
            .collect::<Vec<_>>(),
        vec![(1, 0x120, 0), (0, 0, 0)]
    );
    drop(script);
    output.release().unwrap();
}
