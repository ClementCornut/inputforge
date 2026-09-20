use super::fixtures::*;
use crate::device::evdev::{Access, StreamUpdate};

#[test]
fn passive_streams_all_readable_controllers_without_grabs_and_reuses_handles() {
    let fake = world();
    fake.borrow_mut().devices[2].access = Access::Failed;
    let mut virtual_device = device("virtual");
    virtual_device.classification.kind = crate::device::evdev::Class::Excluded;
    fake.borrow_mut().devices.push(virtual_device);
    let mut capture = capture(&fake);
    capture.start_monitoring().unwrap();
    let mut updates = Vec::new();
    for _ in 0..3 {
        capture.poll_monitor(&mut updates).unwrap();
    }
    assert!(fake.borrow().grabbed.is_empty());
    assert_eq!(fake.borrow().opened.len(), 2);
    assert_eq!(
        updates
            .iter()
            .filter(|u| matches!(u, StreamUpdate::Snapshot { .. }))
            .count(),
        2
    );
    capture.acquire([id("a")]).unwrap();
    assert_eq!(fake.borrow().grabbed.len(), 1);
    capture.release().unwrap();
    assert!(fake.borrow().grabbed.is_empty());
    assert_eq!(fake.borrow().opened.len(), 2);
    assert_eq!(
        fake.borrow()
            .calls
            .iter()
            .filter(|c| c.starts_with("open read-only"))
            .count(),
        2
    );
}

#[test]
fn passive_grab_failure_rolls_back_and_unrelated_disconnect_keeps_capture() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.start_monitoring().unwrap();
    fail(&fake, "grab", "b");
    capture.acquire([id("a"), id("b")]).unwrap_err();
    assert!(fake.borrow().grabbed.is_empty());
    assert_eq!(fake.borrow().opened.len(), 3);
    capture.acquire([id("a")]).unwrap();
    fake.borrow_mut()
        .devices
        .retain(|d| d.identity.id != Some(id("b")));
    fake.borrow_mut().pending = 1;
    let mut updates = Vec::new();
    capture.poll_monitor(&mut updates).unwrap();
    assert_eq!(capture.captured(), [id("a")]);
    assert!(updates.contains(&StreamUpdate::Reset { device: id("b") }));
}

#[test]
fn passive_hotplug_opens_once_and_isolates_stream_failure() {
    let fake = world();
    fake.borrow_mut().devices.truncate(1);
    let mut capture = capture(&fake);
    capture.start_monitoring().unwrap();
    fake.borrow_mut().devices.push(device("b"));
    fake.borrow_mut().pending = 1;
    fail(&fake, "read events", "b");
    let mut updates = Vec::new();
    capture.poll_monitor(&mut updates).unwrap();
    assert!(fake.borrow().opened.contains("a"));
    assert!(!fake.borrow().opened.contains("b"));
    assert!(
        updates
            .iter()
            .any(|u| matches!(u, StreamUpdate::Snapshot { device, .. } if device == &id("a")))
    );
}

#[test]
fn adapter_failed_replacement_keeps_monitoring_until_later_success() {
    use crate::device::InputSource;
    use crate::device::evdev::EvdevInput;

    let old = world();
    old.borrow_mut().devices.truncate(1);
    let mut source = EvdevInput::from_capture(capture(&old));

    let replacement = world();
    replacement.borrow_mut().devices.remove(0);
    let failed = capture(&replacement);
    replacement.borrow_mut().scan_error = true;

    source.replace_capture(failed).unwrap_err();
    source.poll(&mut Vec::new()).unwrap();
    assert_eq!(
        source
            .enumerate_devices()
            .into_iter()
            .map(|device| device.id)
            .collect::<Vec<_>>(),
        [id("a")]
    );
    assert!(old.borrow().opened.contains("a"));

    replacement.borrow_mut().scan_error = false;
    source.replace_capture(capture(&replacement)).unwrap();
    assert_eq!(
        source
            .enumerate_devices()
            .into_iter()
            .map(|device| device.id)
            .collect::<Vec<_>>(),
        [id("b"), id("c")]
    );
    assert!(old.borrow().opened.is_empty());
    assert!(replacement.borrow().opened.contains("b"));
}

#[test]
fn adapter_monitors_without_profile_and_emits_native_axis_samples() {
    use crate::device::evdev::{AxisInfo, EvdevInput};
    use crate::device::{InputSource, InputUpdate};
    let fake = world();
    fake.borrow_mut().devices.truncate(1);
    fake.borrow_mut().devices[0].metadata.abs_axes.insert(5);
    fake.borrow_mut().devices[0].axes.push(AxisInfo {
        code: 5,
        minimum: 10,
        maximum: 110,
        fuzz: 0,
        flat: 0,
        resolution: 0,
    });
    fake.borrow_mut()
        .streams
        .entry("a".into())
        .or_default()
        .state
        .axes
        .insert(5, 10);
    let mut source = EvdevInput::from_capture(capture(&fake));
    let mut updates = Vec::new();
    source.poll(&mut updates).unwrap();
    assert!(updates.iter().any(|update| matches!(update, InputUpdate::AxisSample { device, index: 0, value } if device == &id("a") && (*value + 1.0).abs() < f64::EPSILON)));
    assert!(
        updates
            .iter()
            .any(|update| matches!(update, InputUpdate::Snapshot { .. }))
    );
    assert!(fake.borrow().grabbed.is_empty());
    source.request_axis_sample(&id("a"), 0).unwrap();
    updates.clear();
    source.poll(&mut updates).unwrap();
    assert!(matches!(
        updates.as_slice(),
        [InputUpdate::AxisSample { index: 0, .. }]
    ));
    source.acquire(&[id("a")]).unwrap();
    source.release().unwrap();
    updates.clear();
    source.poll(&mut updates).unwrap();
    assert!(
        updates.is_empty(),
        "grabbing and releasing must not restart snapshots"
    );
    assert_eq!(
        fake.borrow()
            .calls
            .iter()
            .filter(|c| c.starts_with("open read-only"))
            .count(),
        1
    );
}

#[test]
fn adapter_reconciles_sparse_codes_without_reusing_missing_slots() {
    use crate::device::evdev::EvdevInput;
    use crate::device::{HotplugEvent, InputSource, InputUpdate};
    use crate::types::{InputAddress, InputId};
    let fake = world();
    fake.borrow_mut().devices.truncate(1);
    fake.borrow_mut().devices[0].metadata.keys = [300, 704].into();
    fake.borrow_mut()
        .streams
        .entry("a".into())
        .or_default()
        .state
        .keys = [(300, false), (704, true)].into();
    let mut source = EvdevInput::from_capture(capture(&fake));
    source.poll(&mut Vec::new()).unwrap();
    source.hotplug_events();
    fake.borrow_mut().devices[0].metadata.keys = [200, 704].into();
    fake.borrow_mut().streams.get_mut("a").unwrap().state.keys = [(200, true), (704, false)].into();
    fake.borrow_mut().pending = 1;
    let mut updates = Vec::new();
    source.poll(&mut updates).unwrap();
    let table = source.binding_table(&id("a")).unwrap().unwrap();
    assert_eq!(table.buttons, [300, 704, 200]);
    assert_eq!(table.unavailable, [InputId::Button { index: 0 }]);
    assert!(
        source.hotplug_events().iter().any(
            |event| matches!(event, HotplugEvent::Connected { info, .. } if info.buttons == 3)
        )
    );
    let values = updates
        .iter()
        .find_map(|update| match update {
            InputUpdate::Snapshot { values, .. } => Some(values),
            _ => None,
        })
        .unwrap();
    assert_eq!(values.len(), 2);
    assert!(values.iter().any(|event| matches!(
        event.source,
        InputAddress::Bound {
            input: InputId::Button { index: 2 },
            ..
        }
    )));
}

#[test]
fn adapter_announces_removed_controls_even_when_slot_counts_do_not_change() {
    use crate::device::evdev::EvdevInput;
    use crate::device::{HotplugEvent, InputSource};
    use crate::types::InputId;
    let fake = world();
    fake.borrow_mut().devices.truncate(1);
    fake.borrow_mut().devices[0].metadata.keys = [300, 704].into();
    fake.borrow_mut()
        .streams
        .entry("a".into())
        .or_default()
        .state
        .keys = [(300, false), (704, false)].into();
    let mut source = EvdevInput::from_capture(capture(&fake));
    source.poll(&mut Vec::new()).unwrap();
    source.hotplug_events();
    fake.borrow_mut().devices[0].metadata.keys.remove(&300);
    fake.borrow_mut()
        .streams
        .get_mut("a")
        .unwrap()
        .state
        .keys
        .remove(&300);
    fake.borrow_mut().pending = 1;
    source.poll(&mut Vec::new()).unwrap();
    assert!(
        source
            .hotplug_events()
            .iter()
            .any(|e| matches!(e, HotplugEvent::Connected { info, .. } if info.buttons == 2))
    );
    assert_eq!(
        source.binding_table(&id("a")).unwrap().unwrap().unavailable,
        [InputId::Button { index: 0 }]
    );
}

#[test]
fn unrelated_recovery_does_not_suspend_selected_controller_and_reads_stay_bounded() {
    let fake = world();
    fake.borrow_mut().devices.truncate(2);
    let mut capture = capture(&fake);
    capture.start_monitoring().unwrap();
    for _ in 0..2 {
        capture.poll_monitor(&mut Vec::new()).unwrap();
    }
    capture.acquire([id("a")]).unwrap();
    fake.borrow_mut()
        .streams
        .get_mut("b")
        .unwrap()
        .reads
        .push_back(Ok(vec![evdev::InputEvent::new(0, 3, 0)]));
    let mut updates = Vec::new();
    assert!(capture.poll_monitor(&mut updates).unwrap().ready);
    assert!(updates.contains(&StreamUpdate::Reset { device: id("b") }));
    let before: usize = fake
        .borrow()
        .streams
        .values()
        .map(|stream| stream.reads_count)
        .sum();
    for _ in 0..10 {
        fake.borrow_mut()
            .streams
            .get_mut("a")
            .unwrap()
            .reads
            .push_back(Ok(vec![evdev::InputEvent::new(4, 0, 0); 32]));
    }
    assert!(capture.poll_monitor(&mut Vec::new()).unwrap().pending);
    let after: usize = fake
        .borrow()
        .streams
        .values()
        .map(|stream| stream.reads_count)
        .sum();
    assert_eq!(after - before, 8);
}

#[test]
fn selected_stream_fault_preserves_passive_snapshots_and_delivers_reset_on_next_poll() {
    use crate::device::evdev::EvdevInput;
    use crate::device::{InputSource, InputUpdate};
    let fake = world();
    fake.borrow_mut().devices.truncate(2);
    let mut source = EvdevInput::from_capture(capture(&fake));
    source.acquire(&[id("a"), id("b")]).unwrap();
    fail(&fake, "read events", "b");
    let mut updates = Vec::new();
    source.poll(&mut updates).unwrap_err();
    assert!(updates.is_empty());
    assert!(fake.borrow().grabbed.is_empty());
    source.poll(&mut updates).unwrap();
    assert!(
        updates
            .iter()
            .any(|u| matches!(u, InputUpdate::Snapshot { device, .. } if device == &id("a")))
    );
    assert!(
        updates
            .iter()
            .any(|u| matches!(u, InputUpdate::Reset { device } if device == &id("b")))
    );
}

#[test]
fn selected_stream_failure_preserves_ungrab_error_and_passive_survivors() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.start_monitoring().unwrap();
    capture.acquire([id("a")]).unwrap();
    fail(&fake, "read events", "a");
    fail(&fake, "ungrab", "a");
    let error = capture.poll_monitor(&mut Vec::new()).unwrap_err();
    assert_eq!(error.operation(), "poll stream");
    assert_eq!(error.cleanup_failures().len(), 1);
    assert!(fake.borrow().grabbed.is_empty());
    assert!(!fake.borrow().opened.contains("a"));
    assert!(fake.borrow().opened.contains("b"));
}
