use super::fixtures::{capture, device, id, released, world};
use std::time::Duration;

#[test]
fn notifications_and_periodic_reconciliation_refresh_inventory() {
    let fake = world();
    let mut capture = capture(&fake);
    fake.borrow_mut().devices.clear();
    fake.borrow_mut().pending = 2;
    capture.poll().unwrap();
    assert!(capture.devices().is_empty());
    fake.borrow_mut().devices.push(device("a"));
    fake.borrow_mut().now += Duration::from_secs(1);
    capture.poll().unwrap();
    assert_eq!(capture.devices().len(), 1);
    assert!(capture.captured().is_empty());
}

#[test]
fn backlog_is_bounded_and_prevents_acquisition_until_drained() {
    let fake = world();
    let mut capture = capture(&fake);
    fake.borrow_mut().pending = 600;
    assert!(capture.acquire([id("a")]).is_err());
    assert_eq!(fake.borrow().pending, 344);
    released(&fake);
    capture.poll().unwrap();
    assert_eq!(fake.borrow().pending, 88);
    capture.poll().unwrap();
    capture.acquire([id("a")]).unwrap();
}

#[test]
fn selected_disconnect_releases_survivors_without_udev_notification() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.acquire([id("a"), id("b")]).unwrap();
    fake.borrow_mut().dead.insert("a".into());
    assert!(capture.poll().is_err());
    released(&fake);
    fake.borrow_mut().dead.clear();
    assert!(capture.acquire([id("a")]).is_err());
    assert!(capture.poll().is_err());
}

#[test]
fn unrelated_disconnect_preserves_capture() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.acquire([id("a")]).unwrap();
    fake.borrow_mut()
        .devices
        .retain(|device| device.identity.id != Some(id("b")));
    fake.borrow_mut().pending = 1;
    capture.poll().unwrap();
    assert_eq!(capture.captured(), &[id("a")]);
}

#[test]
fn selected_collision_or_same_path_replacement_invalidates_capture() {
    for replace_node in [false, true] {
        let fake = world();
        let mut capture = capture(&fake);
        capture.acquire([id("a")]).unwrap();
        if replace_node {
            fake.borrow_mut().generation += 1;
        } else {
            fake.borrow_mut().devices[0].identity.id = None;
        }
        fake.borrow_mut().pending = 1;
        assert!(capture.poll().is_err());
        released(&fake);
    }
}

#[test]
fn fatal_monitor_or_scan_failure_releases_and_invalidates_owner() {
    for monitor in [false, true] {
        let fake = world();
        let mut capture = capture(&fake);
        capture.acquire([id("a"), id("b")]).unwrap();
        if monitor {
            fake.borrow_mut().monitor_error = true;
        } else {
            fake.borrow_mut().scan_error = true;
            fake.borrow_mut().pending = 1;
        }
        assert!(capture.poll().is_err());
        released(&fake);
        fake.borrow_mut().monitor_error = false;
        fake.borrow_mut().scan_error = false;
        assert!(capture.acquire([id("a")]).is_err());
        capture.release().unwrap();
    }
}

#[test]
fn reconnected_inventory_uses_stable_id_without_automatic_capture() {
    let fake = world();
    let mut capture = capture(&fake);
    fake.borrow_mut().devices.clear();
    fake.borrow_mut().pending = 1;
    capture.poll().unwrap();
    let mut reconnected = device("a");
    reconnected.metadata.node = "/fixture/new-event".into();
    fake.borrow_mut().devices.push(reconnected);
    fake.borrow_mut().pending = 1;
    capture.poll().unwrap();
    assert_eq!(capture.devices()[0].identity.id, Some(id("a")));
    assert!(capture.captured().is_empty());
    released(&fake);
}

#[test]
fn delayed_metadata_and_collision_resolution_restore_only_eligibility() {
    let fake = world();
    fake.borrow_mut().devices[0].identity.id = None;
    let mut capture = capture(&fake);
    assert!(capture.acquire([id("a")]).is_err());
    fake.borrow_mut().devices[0] = device("a");
    fake.borrow_mut().now += Duration::from_secs(1);
    capture.poll().unwrap();
    assert_eq!(capture.devices()[0].identity.id, Some(id("a")));
    assert!(capture.captured().is_empty());
    capture.acquire([id("a")]).unwrap();
}

#[test]
fn selected_metadata_changes_release_capture_but_unselected_errors_do_not() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.acquire([id("a")]).unwrap();
    fake.borrow_mut().devices[1].access = crate::device::evdev::Access::Failed;
    fake.borrow_mut().pending = 1;
    capture.poll().unwrap();
    assert_eq!(capture.captured(), &[id("a")]);
    fake.borrow_mut().devices[0].metadata.name = "replacement".into();
    fake.borrow_mut().pending = 1;
    let error = capture.poll().unwrap_err();
    assert_eq!(error.operation(), "verify captured inventory");
    released(&fake);
}

#[test]
fn poll_preserves_primary_failure_and_reports_all_cleanup_failures() {
    let fake = world();
    let mut capture = capture(&fake);
    capture.acquire([id("a"), id("b")]).unwrap();
    super::fixtures::fail(&fake, "ungrab", "a");
    super::fixtures::fail(&fake, "ungrab", "b");
    fake.borrow_mut().monitor_error = true;
    let error = capture.poll().unwrap_err();
    assert_eq!(error.operation(), "poll udev");
    assert_eq!(error.raw_os_error(), Some(5));
    assert_eq!(error.cleanup_failures().len(), 2);
    released(&fake);
    assert!(capture.captured().is_empty());
    capture.release().unwrap();
}
