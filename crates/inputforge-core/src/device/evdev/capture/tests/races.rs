use super::fixtures::{capture, device, id, released, world};
use crate::device::evdev::{Access, Issue};

#[test]
fn notification_during_final_scan_prevents_commit_and_allows_explicit_retry() {
    let fake = world();
    let mut capture = capture(&fake);
    fake.borrow_mut().pending_on_scan = Some((3, 1));
    assert!(capture.acquire([id("a"), id("b")]).is_err());
    assert!(capture.captured().is_empty());
    released(&fake);
    capture.poll().unwrap();
    capture.acquire([id("a"), id("b")]).unwrap();
}

#[test]
fn notification_during_initial_scan_is_reconciled_on_first_poll() {
    let fake = world();
    fake.borrow_mut().pending_on_scan = Some((1, 1));
    let mut capture = capture(&fake);
    fake.borrow_mut().devices = vec![device("new")];
    capture.poll().unwrap();
    assert_eq!(capture.devices()[0].identity.id, Some(id("new")));
    assert!(capture.captured().is_empty());
}

#[test]
fn selected_access_error_keeps_discovery_operation_and_errno() {
    let fake = world();
    let mut denied = device("a");
    denied.access = Access::Failed;
    denied.issues.push(Issue::new(
        "open read-only",
        &denied.metadata.node,
        &std::io::Error::from_raw_os_error(13),
    ));
    fake.borrow_mut().devices = vec![denied];
    let mut capture = capture(&fake);
    let error = capture.acquire([id("a")]).unwrap_err();
    assert_eq!(error.operation(), "open read-only");
    assert_eq!(error.raw_os_error(), Some(13));
    assert_eq!(error.device_id(), Some(&id("a")));
    released(&fake);
}

#[test]
fn native_health_poll_ignores_unread_data_and_detects_closed_peer() {
    use super::super::hotplug::{check_health, readiness};
    use nix::poll::PollFlags;
    use std::{fs::File, io::Write};
    let (reader, writer) = nix::unistd::pipe().unwrap();
    let mut writer = File::from(writer);
    writer.write_all(b"unread").unwrap();
    assert!(readiness(&reader, PollFlags::empty()).unwrap().is_empty());
    drop(writer);
    assert!(check_health(readiness(&reader, PollFlags::empty()).unwrap()).is_err());
}
