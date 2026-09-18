use super::{Report, finish};
use inputforge_core::{
    device::evdev::{NativeChange, NativeControl, NativeState, SnapshotKind, StreamUpdate},
    types::DeviceId,
};
use std::{cell::Cell, io, time::Instant};

fn snapshot(kind: SnapshotKind, held: bool) -> StreamUpdate {
    StreamUpdate::Snapshot {
        device: DeviceId("evdev:v1:test".into()),
        state: NativeState {
            keys: [(300, held)].into(),
            axes: [(5, 15)].into(),
        },
        kind,
        timestamp: Instant::now(),
    }
}

fn frame(value: i32) -> StreamUpdate {
    StreamUpdate::Frame {
        device: DeviceId("evdev:v1:test".into()),
        changes: vec![NativeChange {
            control: NativeControl::Abs(5),
            value,
        }],
        timestamp: Instant::now(),
    }
}

fn output(report: &Report) -> String {
    let mut bytes = Vec::new();
    report.write(&mut bytes).expect("report writes to memory");
    String::from_utf8(bytes).expect("report is UTF-8")
}

#[test]
fn sample_cap_does_not_stop_counts_extrema_or_final_state() {
    let mut report = Report::default();
    report.observe(snapshot(SnapshotKind::Initial, true));
    for value in 0..300 {
        report.observe(frame(value));
    }
    let text = output(&report);
    assert!(text.contains("sampled_frames=256 omitted_frames=44"));
    assert!(text.contains("Abs(5) events=300 min=0 max=299"));
    assert!(text.contains("final=Some(NativeState { keys: {300: true}, axes: {5: 299}"));
    assert!(text.contains("Key(300) events=0 min=1 max=1"));
}

#[test]
fn resets_and_failures_invalidate_final_state_but_keep_history() {
    let mut report = Report::default();
    report.observe(snapshot(SnapshotKind::Initial, true));
    report.observe(StreamUpdate::Reset {
        device: DeviceId("evdev:v1:test".into()),
    });
    let text = output(&report);
    assert!(text.contains("resets=1"));
    assert!(text.contains("final=None"));
    assert!(text.contains("initial=Some(NativeState"));
    report.observe(snapshot(SnapshotKind::Recovered, false));
    assert!(output(&report).contains("final=Some(NativeState { keys: {300: false}"));
    report.invalidate();
    assert!(output(&report).contains("final=None"));
}

#[test]
fn output_failure_happens_after_release_even_when_work_failed() {
    let released = Cell::new(false);
    let error = finish(
        Err(io::Error::other("stream failed").into()),
        || {
            released.set(true);
            Err(io::Error::other("release failed").into())
        },
        || {
            assert!(released.get());
            Err(io::ErrorKind::BrokenPipe.into())
        },
    )
    .expect_err("all three failures must be retained");
    let message = error.to_string();
    assert!(message.contains("stream failed"));
    assert!(message.contains("release failed"));
    assert!(message.contains("report:"));
}
