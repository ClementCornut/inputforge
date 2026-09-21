use super::{
    super::{device::DeviceKey, event_device::EventError},
    fixtures,
};
use crate::output::OutputPhase;
use evdev::{InputEvent, KeyCode, RelativeAxisCode};
use std::io;

const CTRL: u16 = KeyCode::KEY_LEFTCTRL.0;
const A: u16 = KeyCode::KEY_A.0;
const B: u16 = KeyCode::KEY_B.0;

fn key_events(world: &fixtures::World) -> Vec<(u16, i32)> {
    world
        .lock()
        .unwrap()
        .emitted
        .iter()
        .flat_map(|(_, events)| events)
        .filter(|event| event.event_type().0 == 1)
        .map(|event| (event.code(), event.value()))
        .collect()
}

#[test]
fn inert_construction_and_idempotent_healthy_start_stop() {
    let (mut device, world) = fixtures::event_device(vec![CTRL, A, B]);
    assert!(world.lock().unwrap().alive.is_empty());

    device.start().unwrap();
    device.start().unwrap();
    assert_eq!(
        world
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|(_, call)| *call == "create")
            .count(),
        1
    );
    device.stop().unwrap();
    device.stop().unwrap();
    assert!(world.lock().unwrap().alive.is_empty());
}

#[test]
fn failed_start_rejects_restart_until_stop() {
    let (mut device, world) = fixtures::event_device(vec![A]);
    world.lock().unwrap().fail_ready = Some((DeviceKey::Keyboard, io::ErrorKind::InvalidData));
    assert_eq!(device.start().unwrap_err().phase, OutputPhase::Readiness);
    assert_eq!(
        device.start().unwrap_err().phase,
        OutputPhase::Initialization
    );
    assert_eq!(
        world
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|(_, call)| *call == "create")
            .count(),
        1
    );

    world.lock().unwrap().fail_ready = None;
    device.stop().unwrap();
    device.start().unwrap();
}

#[test]
fn overlapping_groups_retain_their_shared_modifier() {
    let (mut device, world) = fixtures::started_event_device(vec![CTRL, A, B]);
    device.press(&[CTRL, A]).unwrap();
    device.press(&[CTRL, B]).unwrap();
    device.release(&[CTRL, A]).unwrap();
    device.release(&[CTRL, B]).unwrap();

    assert_eq!(
        key_events(&world),
        [(CTRL, 1), (A, 1), (B, 1), (A, 0), (B, 0), (CTRL, 0)]
    );
}

#[test]
fn duplicate_group_ownership_balances_correctly() {
    let (mut device, world) = fixtures::started_event_device(vec![CTRL, A]);
    device.press(&[CTRL, A]).unwrap();
    device.press(&[CTRL, A]).unwrap();
    device.release(&[CTRL, A]).unwrap();
    assert_eq!(key_events(&world), [(CTRL, 1), (A, 1)]);
    device.release(&[CTRL, A]).unwrap();
    assert_eq!(key_events(&world), [(CTRL, 1), (A, 1), (A, 0), (CTRL, 0)]);
}

#[test]
fn pulse_during_an_existing_hold_emits_no_extra_edge() {
    let (mut device, world) = fixtures::started_event_device(vec![A]);
    device.press(&[A]).unwrap();
    world.lock().unwrap().emitted.clear();

    device.press(&[A]).unwrap();
    device.release(&[A]).unwrap();
    assert!(world.lock().unwrap().emitted.is_empty());
    device.release(&[A]).unwrap();
    assert_eq!(key_events(&world), [(A, 0)]);
    world.lock().unwrap().emitted.clear();
    device.stop().unwrap();
    assert!(world.lock().unwrap().emitted.is_empty());
}

#[test]
fn partial_key_down_and_failed_synchronization_remain_cleanup_candidates() {
    let (mut device, world) = fixtures::started_event_device(vec![CTRL, A]);
    world.lock().unwrap().writes.extend([
        Ok(size_of::<InputEvent>()),
        Err(io::Error::from_raw_os_error(5)),
    ]);
    assert_eq!(
        device.press(&[CTRL, A]).unwrap_err().phase,
        OutputPhase::Emission
    );
    world.lock().unwrap().emitted.clear();

    device.stop().unwrap();
    assert_eq!(key_events(&world), [(CTRL, 0), (A, 0)]);
}

#[test]
fn failed_key_up_is_retried_during_cleanup() {
    let (mut device, world) = fixtures::started_event_device(vec![A]);
    device.press(&[A]).unwrap();
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::WouldBlock.into()));
    assert_eq!(
        device.release(&[A]).unwrap_err().phase,
        OutputPhase::Release
    );
    world.lock().unwrap().emitted.clear();

    device.stop().unwrap();
    assert_eq!(key_events(&world), [(A, 0)]);
}

#[test]
fn release_after_failed_key_up_and_stop_allows_restart() {
    let (mut device, world) = fixtures::started_event_device(vec![A]);
    device.press(&[A]).unwrap();
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::WouldBlock.into()));
    device.release(&[A]).expect_err("scripted key-up failure");

    device.stop().unwrap();
    device
        .release(&[A])
        .expect("stopped device no longer owns the group");
    device.start().expect("fresh device can restart");
}

#[test]
fn cleanup_continues_across_failures_and_destroys_the_descriptor() {
    let (mut device, world) = fixtures::started_event_device(vec![CTRL, A, B]);
    device.press(&[CTRL, A, B]).unwrap();
    world
        .lock()
        .unwrap()
        .writes
        .push_back(Err(io::ErrorKind::WouldBlock.into()));
    world
        .lock()
        .unwrap()
        .fail_destroy
        .insert(DeviceKey::Keyboard);

    let EventError { phase, error } = device.stop().unwrap_err();
    assert_eq!(phase, OutputPhase::Release);
    assert!(!error.cleanup_failures().is_empty());
    let script = world.lock().unwrap();
    assert!(script.alive.is_empty());
    assert!(script.emitted.iter().any(|(_, events)| {
        events
            .iter()
            .any(|event| event.event_type().0 == 1 && event.code() == A && event.value() == 0)
    }));
    assert!(script.emitted.iter().any(|(_, events)| {
        events
            .iter()
            .any(|event| event.event_type().0 == 1 && event.code() == B && event.value() == 0)
    }));
}

#[test]
fn uncertain_wheel_delivery_is_not_replayed() {
    let (mut device, world) = fixtures::started_event_device(vec![]);
    world.lock().unwrap().writes.extend([
        Ok(size_of::<InputEvent>()),
        Err(io::Error::from_raw_os_error(5)),
    ]);
    assert_eq!(
        device
            .relative(RelativeAxisCode::REL_WHEEL.0, 1)
            .unwrap_err()
            .phase,
        OutputPhase::Emission
    );
    world.lock().unwrap().emitted.clear();

    device.stop().unwrap();
    assert!(world.lock().unwrap().emitted.is_empty());
}
