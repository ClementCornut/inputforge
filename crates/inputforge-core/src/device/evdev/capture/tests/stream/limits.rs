use super::super::fixtures::{capture, id, released};
use super::fixtures::{event, queue, ready, report, setup};
use std::{io::ErrorKind, time::Duration};

#[test]
fn read_attempt_and_snapshot_limits_hold_without_starving_devices() {
    let (fake, mut owner) = setup(&["a", "b", "c"]);
    let mut out = Vec::new();
    for expected in 1..=3 {
        owner.poll_stream(&mut out).unwrap();
        assert_eq!(
            fake.borrow()
                .streams
                .values()
                .map(|s| s.snapshots)
                .sum::<usize>(),
            expected
        );
    }
    assert!(owner.stream_state(&id("c")).is_some());
    for name in ["a", "b", "c"] {
        for _ in 0..20 {
            queue(&fake, name, vec![report()]);
        }
    }
    let before = fake
        .borrow()
        .streams
        .values()
        .map(|s| s.reads_count)
        .sum::<usize>();
    assert!(owner.poll_stream(&mut out).unwrap().pending);
    assert_eq!(
        fake.borrow()
            .streams
            .values()
            .map(|s| s.reads_count)
            .sum::<usize>()
            - before,
        8
    );
    for name in ["a", "b", "c"] {
        assert!(fake.borrow().streams[name].reads.len() < 20);
    }
}

#[test]
fn unfinished_frame_storage_is_bounded_including_ignored_events() {
    for kind in [1, 4] {
        let (fake, mut owner) = setup(&["a"]);
        ready(&fake, &mut owner);
        let mut out = Vec::new();
        for _ in 0..16 {
            queue(&fake, "a", vec![event(kind, 0x120, 1); 256]);
            owner.poll_stream(&mut out).unwrap();
        }
        assert!(out.is_empty());
        queue(&fake, "a", vec![event(kind, 0x120, 1)]);
        assert_eq!(
            owner.poll_stream(&mut out).unwrap_err().issue().kind,
            ErrorKind::InvalidData
        );
        released(&fake);
    }
}

#[test]
fn initialization_deadline_does_not_restart_after_failed_snapshot() {
    let (fake, mut owner) = setup(&["a"]);
    fake.borrow_mut()
        .streams
        .get_mut("a")
        .unwrap()
        .during_snapshot = vec![report()];
    let mut out = Vec::new();
    assert!(!owner.poll_stream(&mut out).unwrap().ready);
    fake.borrow_mut().now += Duration::from_secs(1);
    assert_eq!(
        owner.poll_stream(&mut out).unwrap_err().issue().kind,
        ErrorKind::TimedOut
    );
    assert!(out.is_empty());
    released(&fake);
}

#[test]
fn idle_synchronized_devices_have_no_frame_timeout() {
    let (fake, mut owner) = setup(&["a"]);
    ready(&fake, &mut owner);
    fake.borrow_mut().now += Duration::from_secs(100);
    let status = owner.poll_stream(&mut Vec::new()).unwrap();
    assert!(status.ready);
    assert!(!status.pending);
}

#[test]
fn relative_and_multitouch_are_rejected_only_when_streaming_is_requested() {
    for protocol in 0..3 {
        let (fake, mut previous) = setup(&["a"]);
        previous.release().unwrap();
        let mut world = fake.borrow_mut();
        match protocol {
            0 => {
                world.devices[0].metadata.event_types.insert(2);
            }
            1 => {
                world.devices[0].metadata.rel_axes.insert(0);
            }
            _ => {
                world.devices[0].metadata.abs_axes.insert(0x2f);
            }
        }
        drop(world);
        let mut owner = capture(&fake);
        owner.acquire([id("a")]).unwrap();
        owner.poll().unwrap();
        assert_eq!(
            owner.poll_stream(&mut Vec::new()).unwrap_err().issue().kind,
            ErrorKind::Unsupported
        );
        released(&fake);
    }
}

#[test]
fn button_only_and_axis_only_devices_can_initialize() {
    for buttons in [true, false] {
        let (fake, mut previous) = setup(&["a"]);
        previous.release().unwrap();
        let mut world = fake.borrow_mut();
        if buttons {
            world.devices[0].metadata.abs_axes.clear();
            world.devices[0].axes.clear();
            world.streams.get_mut("a").unwrap().state.axes.clear();
        } else {
            world.devices[0].metadata.keys.clear();
            world.streams.get_mut("a").unwrap().state.keys.clear();
        }
        drop(world);
        let mut owner = capture(&fake);
        owner.acquire([id("a")]).unwrap();
        assert!(owner.poll_stream(&mut Vec::new()).unwrap().ready);
        assert_eq!(
            owner.stream_state(&id("a")).unwrap().keys.is_empty(),
            !buttons
        );
    }
}
