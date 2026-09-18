use super::super::fixtures::{id, released};
use super::fixtures::{event, frames, queue, ready, report, setup};
use crate::device::evdev::{NativeControl, SnapshotKind, StreamUpdate};

#[test]
fn initial_snapshot_includes_held_sparse_button_and_untouched_axes() {
    let (_, mut capture) = setup(&["a"]);
    let mut out = Vec::new();
    assert!(capture.stream_state(&id("a")).is_none());
    assert!(capture.poll_stream(&mut out).unwrap().ready);
    let [
        StreamUpdate::Snapshot {
            state,
            kind: SnapshotKind::Initial,
            ..
        },
    ] = out.as_slice()
    else {
        panic!("expected only initial snapshot: {out:?}");
    };
    assert_eq!(state.keys.get(&0x2c0), Some(&true));
    assert_eq!(state.axes.get(&0), Some(&123));
}

#[test]
fn split_frames_preserve_transitions_and_hide_incomplete_state() {
    let (fake, mut capture) = setup(&["a"]);
    ready(&fake, &mut capture);
    queue(&fake, "a", vec![event(1, 0x2c0, 0), event(3, 0x10, 1)]);
    let mut out = Vec::new();
    assert!(capture.poll_stream(&mut out).unwrap().pending);
    assert!(out.is_empty());
    assert!(capture.stream_state(&id("a")).unwrap().keys[&0x2c0]);
    queue(
        &fake,
        "a",
        vec![event(1, 0x2c0, 1), event(1, 0x2c0, 2), report()],
    );
    capture.poll_stream(&mut out).unwrap();
    let [StreamUpdate::Frame { changes, .. }] = out.as_slice() else {
        panic!("{out:?}")
    };
    assert_eq!(
        changes
            .iter()
            .map(|c| (c.control, c.value))
            .collect::<Vec<_>>(),
        [
            (NativeControl::Key(0x2c0), 0),
            (NativeControl::Abs(0x10), 1),
            (NativeControl::Key(0x2c0), 1)
        ]
    );
    assert_eq!(capture.stream_state(&id("a")).unwrap().axes[&0x10], 1);
}

#[test]
fn budgets_preserve_fetched_tail_and_service_other_devices() {
    let (fake, mut capture) = setup(&["a", "b"]);
    ready(&fake, &mut capture);
    queue(
        &fake,
        "a",
        (0..100).flat_map(|n| [event(3, 0, n), report()]).collect(),
    );
    queue(
        &fake,
        "b",
        (0..100).flat_map(|n| [event(3, 0, n), report()]).collect(),
    );
    let mut out = Vec::new();
    assert!(capture.poll_stream(&mut out).unwrap().pending);
    assert_eq!(frames(&out), 128);
    capture.poll_stream(&mut out).unwrap();
    assert_eq!(frames(&out), 200);
    assert_eq!(capture.stream_state(&id("a")).unwrap().axes[&0], 99);
    assert_eq!(capture.stream_state(&id("b")).unwrap().axes[&0], 99);
}

#[test]
fn fatal_batch_or_control_error_discards_this_calls_updates_and_releases_all() {
    for bad in [
        vec![report(); 257],
        vec![event(1, 0x120, 9)],
        vec![event(3, 7, 1)],
        vec![],
    ] {
        let (fake, mut capture) = setup(&["a", "b"]);
        ready(&fake, &mut capture);
        queue(&fake, "a", vec![event(3, 0, 7), report()]);
        queue(&fake, "b", bad);
        let prefix = StreamUpdate::Reset { device: id("old") };
        let mut out = vec![prefix.clone()];
        capture.poll_stream(&mut out).unwrap_err();
        assert_eq!(out, [prefix]);
        assert!(capture.stream_state(&id("a")).is_none());
        released(&fake);
        assert!(capture.acquire([id("a")]).is_err());
    }
}

#[test]
fn interrupted_reads_yield_and_lifecycle_only_poll_never_reads() {
    let (fake, mut capture) = setup(&["a"]);
    capture.poll().unwrap();
    assert_eq!(fake.borrow().streams["a"].reads_count, 0);
    fake.borrow_mut()
        .streams
        .get_mut("a")
        .unwrap()
        .reads
        .push_back(Err(4));
    let mut out = Vec::new();
    assert!(!capture.poll_stream(&mut out).unwrap().ready);
    assert!(out.is_empty());
    assert!(capture.poll_stream(&mut out).unwrap().ready);
}
