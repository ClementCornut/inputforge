use super::super::fixtures::{id, released};
use super::fixtures::{dropped, event, frames, queue, ready, report, setup};
use crate::device::evdev::{SnapshotKind, StreamUpdate};
use std::time::Duration;

#[test]
fn dropped_release_invalidates_state_until_report_and_replacement_snapshot() {
    let (fake, mut capture) = setup(&["a"]);
    ready(&fake, &mut capture);
    queue(
        &fake,
        "a",
        vec![event(1, 0x120, 1), dropped(), event(3, 0, 999)],
    );
    let mut out = Vec::new();
    assert!(!capture.poll_stream(&mut out).unwrap().ready);
    assert!(matches!(out.as_slice(), [StreamUpdate::Reset { .. }]));
    assert!(capture.stream_state(&id("a")).is_none());
    fake.borrow_mut()
        .streams
        .get_mut("a")
        .unwrap()
        .state
        .keys
        .insert(0x2c0, false);
    queue(
        &fake,
        "a",
        vec![dropped(), report(), event(3, 0, -777), report()],
    );
    out.clear();
    capture.poll_stream(&mut out).unwrap();
    assert_eq!(frames(&out), 0);
    assert!(matches!(out.first(), Some(StreamUpdate::Reset { .. })));
    assert!(matches!(
        out.last(),
        Some(StreamUpdate::Snapshot {
            kind: SnapshotKind::Recovered,
            ..
        })
    ));
    let state = capture.stream_state(&id("a")).unwrap();
    assert!(!state.keys[&0x2c0]);
    assert!(!state.keys[&0x120]);
    assert_eq!(state.axes[&0], 123);
}

#[test]
fn dropped_markers_during_initialization_are_observable_without_extending_deadline() {
    let (fake, mut capture) = setup(&["a"]);
    let mut out = Vec::new();
    queue(&fake, "a", vec![dropped()]);
    assert!(!capture.poll_stream(&mut out).unwrap().ready);
    assert!(matches!(out.as_slice(), [StreamUpdate::Reset { .. }]));
    fake.borrow_mut().now += Duration::from_millis(500);
    queue(&fake, "a", vec![dropped()]);
    capture.poll_stream(&mut out).unwrap();
    assert_eq!(out.len(), 2);
    fake.borrow_mut().now += Duration::from_millis(500);
    capture.poll_stream(&mut out).unwrap_err();
    released(&fake);
}

#[test]
fn events_arriving_during_snapshot_delay_publication_and_are_not_replayed() {
    let (fake, mut capture) = setup(&["a"]);
    fake.borrow_mut()
        .streams
        .get_mut("a")
        .unwrap()
        .during_snapshot = vec![event(3, 0, -999), report()];
    let mut out = Vec::new();
    assert!(!capture.poll_stream(&mut out).unwrap().ready);
    assert!(out.is_empty());
    assert!(capture.poll_stream(&mut out).unwrap().ready);
    assert_eq!(frames(&out), 0);
    assert_eq!(capture.stream_state(&id("a")).unwrap().axes[&0], 123);
}

#[test]
fn missing_frame_delimiter_and_failed_recovery_time_out_and_release() {
    for recovery in [false, true] {
        let (fake, mut capture) = setup(&["a"]);
        ready(&fake, &mut capture);
        queue(
            &fake,
            "a",
            vec![if recovery {
                dropped()
            } else {
                event(1, 0x120, 1)
            }],
        );
        let mut out = Vec::new();
        capture.poll_stream(&mut out).unwrap();
        fake.borrow_mut().now += Duration::from_secs(1);
        let error = capture.poll_stream(&mut out).unwrap_err();
        assert_eq!(error.issue().kind, std::io::ErrorKind::TimedOut);
        released(&fake);
    }
}
