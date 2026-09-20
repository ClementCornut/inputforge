use super::super::native::write::packet;
use evdev::InputEvent;
use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};

#[test]
fn aligned_short_writes_resume_without_replaying_accepted_records() {
    let events = [
        InputEvent::new(1, 0x120, 1),
        InputEvent::new(3, 0, 17),
        InputEvent::new(0, 0, 0),
    ];
    let size = size_of::<InputEvent>();
    let now = Instant::now();
    let mut calls = Vec::new();
    packet(
        &events,
        |rest| {
            calls.push(rest.to_vec());
            Ok(size)
        },
        || now,
        now + Duration::from_millis(50),
    )
    .unwrap();
    assert_eq!(
        calls,
        vec![events.to_vec(), events[1..].to_vec(), events[2..].to_vec()]
    );
}

#[test]
fn no_progress_misalignment_overcounts_and_would_block_fail_promptly() {
    let event = [InputEvent::new(0, 0, 0)];
    let now = Instant::now();
    for bytes in [0, 1, usize::MAX] {
        let mut calls = 0;
        assert!(
            packet(
                &event,
                |_| {
                    calls += 1;
                    Ok(bytes)
                },
                || now,
                now + Duration::from_secs(1)
            )
            .is_err()
        );
        assert_eq!(calls, 1);
    }
    let mut calls = 0;
    let error = packet(
        &event,
        |_| {
            calls += 1;
            Err(io::ErrorKind::WouldBlock.into())
        },
        || now,
        now + Duration::from_secs(1),
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(calls, 1);
}

#[test]
fn interrupted_writes_consume_attempts_and_deadlines_prevent_further_writes() {
    let event = [InputEvent::new(0, 0, 0)];
    let now = Instant::now();
    let mut calls = 0;
    assert!(
        packet(
            &event,
            |_| {
                calls += 1;
                Err(io::ErrorKind::Interrupted.into())
            },
            || now,
            now + Duration::from_secs(1)
        )
        .is_err()
    );
    assert_eq!(calls, 4);
    let mut clock = VecDeque::from([now, now + Duration::from_secs(2)]);
    let mut writes = 0;
    let error = packet(
        &event,
        |_| {
            writes += 1;
            Err(io::ErrorKind::Interrupted.into())
        },
        || clock.pop_front().unwrap(),
        now + Duration::from_secs(1),
    )
    .unwrap_err();
    assert_eq!(writes, 1);
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
}
