mod read;
mod recovery;
mod state;
pub(super) use recovery::Stream;
pub use state::{
    NativeChange, NativeControl, NativeHat, NativeState, SnapshotKind, StreamStatus, StreamUpdate,
};

use super::{Capture, CaptureError, transaction};
use crate::types::DeviceId;
use recovery::Phase;
use std::io;

// These work budgets leave lifecycle/deadline control with the caller under event floods.
const READ_BUDGET: usize = 8;
const EVENT_BUDGET: usize = 256;

impl Capture {
    /// Poll bounded native frames, initializing state on the first call after acquisition.
    ///
    /// Snapshots replace state; they are not button edges. `Reset` invalidates a device
    /// until the next snapshot. Callers must invalidate the entire selection on error.
    /// Each call appends only successful results, preserving the caller's prior entries.
    /// No acquisition, profile translation, calibration or output creation occurs here.
    /// Synchronous kernel calls and discovery scans prevent hard realtime guarantees.
    ///
    /// # Errors
    /// Returns a state error without I/O if no selection is acquired. All stream or
    /// lifecycle failures discard this call's updates, release every selected descriptor,
    /// and invalidate the owner. Initialization/recovery and unfinished frames time out
    /// after one second. Unsupported relative/multitouch protocols are rejected.
    pub fn poll_stream(
        &mut self,
        out: &mut Vec<StreamUpdate>,
    ) -> Result<StreamStatus, CaptureError> {
        self.ensure_valid()?;
        if self.held.is_empty() {
            return Err(CaptureError::state(
                "stream",
                "acquire a selection before streaming",
            ));
        }
        let start = out.len();
        match self.stream_tick(out) {
            Ok(status) => Ok(status),
            Err(mut error) => {
                out.truncate(start);
                self.valid = false;
                if let Err(cleanup) = self.release() {
                    error.append(cleanup);
                }
                Err(error)
            }
        }
    }

    /// Borrow complete native state only while the selected device is synchronized.
    #[must_use]
    pub fn stream_state(&self, id: &DeviceId) -> Option<&NativeState> {
        if !self.valid {
            return None;
        }
        self.held
            .iter()
            .find(|held| held.info.identity.id.as_ref() == Some(id))?
            .stream
            .as_ref()
            .filter(|s| s.ready())
            .map(|s| &s.state)
    }

    fn stream_tick(&mut self, out: &mut Vec<StreamUpdate>) -> Result<StreamStatus, CaptureError> {
        self.poll()?;
        let now = self.monitor.now();
        for held in &mut self.held {
            if held.stream.is_none() {
                held.stream = Some(
                    Stream::new(&held.info, now)
                        .map_err(|e| CaptureError::device("initialize stream", &held.info, e))?,
                );
            }
        }
        let mut reads = READ_BUDGET;
        let mut records = EVENT_BUDGET;
        let mut snapshot = true;
        let mut idle = 0;
        while reads > 0 && records > 0 && idle < self.held.len() {
            let index = self.stream_cursor % self.held.len();
            self.stream_cursor = (index + 1) % self.held.len();
            let held = &mut self.held[index];
            let progress = advance(held, now, &mut reads, &mut records, &mut snapshot, out)
                .map_err(|e| CaptureError::device("poll stream", &held.info, e))?;
            idle = if progress { 0 } else { idle + 1 };
        }
        transaction::verify(&self.held, &self.devices)?;
        let now = self.monitor.now();
        let mut status = StreamStatus {
            ready: true,
            pending: false,
        };
        for held in &self.held {
            let stream = held.stream.as_ref().expect("initialized stream");
            stream
                .check_deadline(now)
                .map_err(|e| CaptureError::device("poll stream", &held.info, e))?;
            status.ready &= stream.ready();
            status.pending |= stream.pending()
                || held
                    .handle
                    .stream_pending()
                    .map_err(|e| CaptureError::device("poll stream descriptor", &held.info, e))?;
        }
        Ok(status)
    }
}

fn advance(
    held: &mut transaction::Held,
    now: std::time::Instant,
    reads: &mut usize,
    records: &mut usize,
    snapshot: &mut bool,
    out: &mut Vec<StreamUpdate>,
) -> io::Result<bool> {
    let stream = held.stream.as_mut().expect("initialized stream");
    stream.check_deadline(now)?;
    if stream.queue.is_empty() {
        *reads -= 1;
        match held.handle.read_events() {
            Ok(events) => stream.queue = events,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => return Ok(false),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if let Some(kind) = stream.snapshot_kind().filter(|_| *snapshot) {
                    *snapshot = false;
                    let state = held.handle.snapshot(&held.info)?;
                    if !state
                        .keys
                        .keys()
                        .copied()
                        .eq(held.info.metadata.keys.iter().copied())
                        || !state.axes.keys().copied().eq(held
                            .info
                            .metadata
                            .abs_axes
                            .iter()
                            .copied())
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "snapshot capabilities changed",
                        ));
                    }
                    if !held.handle.stream_pending()? {
                        stream.state = state.clone();
                        stream.phase = Phase::Ready;
                        out.push(StreamUpdate::Snapshot {
                            device: held.info.identity.id.clone().expect("captured identity"),
                            state,
                            kind,
                            timestamp: now,
                        });
                    }
                }
                return Ok(false);
            }
            Err(e) => return Err(e),
        }
    }
    while *records > 0 {
        let Some(event) = stream.queue.pop_front() else {
            break;
        };
        *records -= 1;
        stream.accept(event, &held.info, now, out)?;
    }
    Ok(true)
}
