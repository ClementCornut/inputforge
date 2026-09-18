use super::{NativeChange, NativeControl, NativeState, SnapshotKind, StreamUpdate};
use crate::device::evdev::Device;
use evdev::InputEvent;
use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};

// Limits apply only to unfinished work, never to a quiet, synchronized controller.
const DEADLINE: Duration = Duration::from_secs(1);
const FRAME_LIMIT: usize = 4096;

#[derive(Debug)]
pub(super) enum Phase {
    Drain {
        kind: SnapshotKind,
        since: Instant,
        wait_report: bool,
    },
    Ready,
}

#[derive(Debug)]
pub(in super::super) struct Stream {
    pub(super) state: NativeState,
    pub(super) phase: Phase,
    pub(super) queue: VecDeque<InputEvent>,
    changes: Vec<NativeChange>,
    frame_since: Option<Instant>,
    records: usize,
}

impl Stream {
    pub(in super::super) fn new(info: &Device, now: Instant) -> io::Result<Self> {
        // EV_REL and ABS_MT_SLOT..ABS_MAX need protocols beyond controller state.
        if info.metadata.event_types.contains(&2)
            || !info.metadata.rel_axes.is_empty()
            || info.metadata.abs_axes.iter().any(|code| *code >= 0x2f)
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "relative/multitouch streaming is unsupported",
            ));
        }
        Ok(Self {
            state: NativeState::default(),
            phase: Phase::Drain {
                kind: SnapshotKind::Initial,
                since: now,
                wait_report: false,
            },
            queue: VecDeque::new(),
            changes: Vec::new(),
            frame_since: None,
            records: 0,
        })
    }

    pub(super) fn ready(&self) -> bool {
        matches!(self.phase, Phase::Ready)
    }

    pub(super) fn pending(&self) -> bool {
        !self.ready() || !self.queue.is_empty() || self.frame_since.is_some()
    }

    pub(super) fn check_deadline(&self, now: Instant) -> io::Result<()> {
        let since = match self.phase {
            Phase::Drain { since, .. } => Some(since),
            Phase::Ready => self.frame_since,
        };
        if since.is_some_and(|since| now.duration_since(since) >= DEADLINE) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "stream initialization, recovery or unfinished frame exceeded one second",
            ));
        }
        Ok(())
    }

    pub(super) fn snapshot_kind(&self) -> Option<SnapshotKind> {
        match self.phase {
            Phase::Drain {
                kind,
                wait_report: false,
                ..
            } => Some(kind),
            _ => None,
        }
    }

    pub(super) fn accept(
        &mut self,
        event: InputEvent,
        info: &Device,
        now: Instant,
        out: &mut Vec<StreamUpdate>,
    ) -> io::Result<()> {
        let id = || info.identity.id.clone().expect("captured identity");
        // EV_SYN / SYN_DROPPED invalidates the unfinished frame immediately.
        if event.event_type().0 == 0 && event.code() == 3 {
            if self.ready() {
                self.phase = Phase::Drain {
                    kind: SnapshotKind::Recovered,
                    since: now,
                    wait_report: true,
                };
            } else if let Phase::Drain { wait_report, .. } = &mut self.phase {
                *wait_report = true;
            }
            out.push(StreamUpdate::Reset { device: id() });
            self.state = NativeState::default();
            self.clear_frame();
            return Ok(());
        }
        let report = event.event_type().0 == 0 && event.code() == 0;
        if let Phase::Drain { wait_report, .. } = &mut self.phase {
            if report {
                *wait_report = false;
            }
            return Ok(());
        }
        if report {
            for change in &self.changes {
                match change.control {
                    NativeControl::Key(code) => {
                        self.state.keys.insert(code, change.value == 1);
                    }
                    NativeControl::Abs(code) => {
                        self.state.axes.insert(code, change.value);
                    }
                }
            }
            if !self.changes.is_empty() {
                out.push(StreamUpdate::Frame {
                    device: id(),
                    changes: std::mem::take(&mut self.changes),
                    timestamp: now,
                });
            }
            self.clear_frame();
            return Ok(());
        }
        self.frame_since.get_or_insert(now);
        self.records += 1;
        if self.records > FRAME_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unfinished frame exceeds 4096 records",
            ));
        }
        let code = event.code();
        let value = event.value();
        let control = match event.event_type().0 {
            1 if info.metadata.keys.contains(&code) && (0..=2).contains(&value) => {
                if value == 2 {
                    return Ok(());
                }
                NativeControl::Key(code)
            }
            3 if info.metadata.abs_axes.contains(&code) => NativeControl::Abs(code),
            1 | 3 => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid or undeclared event type={} code={code:#06x} value={value}",
                        event.event_type().0
                    ),
                ));
            }
            _ => return Ok(()),
        };
        self.changes.push(NativeChange { control, value });
        Ok(())
    }

    fn clear_frame(&mut self) {
        self.changes.clear();
        self.frame_since = None;
        self.records = 0;
    }
}
