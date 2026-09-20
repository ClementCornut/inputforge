//! Passive readers share the same handles, stream budgets and recovery as capture.
use super::{
    Capture, CaptureError, StreamStatus, StreamUpdate,
    handle::Handle,
    stream,
    transaction::{self, Held},
};

impl Capture {
    pub(crate) fn start_monitoring(&mut self) -> Result<(), CaptureError> {
        self.monitoring = true;
        self.refresh(true)?;
        self.open_readers();
        Ok(())
    }

    pub(super) fn open_readers(&mut self) {
        for info in &self.devices {
            let Some(id) = &info.identity.id else {
                continue;
            };
            if self
                .held
                .iter()
                .any(|h| h.info.identity.id.as_ref() == Some(id))
                || transaction::selected(&self.devices, id).is_err()
                || crate::device::evdev::bindings::from_device(info).is_err()
            {
                continue;
            }
            let Ok(handle) = Handle::open(&self.monitor, info) else {
                continue;
            };
            let Ok(stream) = stream::Stream::new(info, self.monitor.now()) else {
                continue;
            };
            self.held.push(Held {
                info: info.clone(),
                handle,
                grabbed: false,
                stream: Some(stream),
            });
        }
    }

    pub(crate) fn poll_monitor(
        &mut self,
        out: &mut Vec<StreamUpdate>,
    ) -> Result<StreamStatus, CaptureError> {
        self.ensure_valid()?;
        let previous_scan = self.scan_generation;
        if let Err(mut error) = self.refresh(false) {
            if let Err(cleanup) = self.release() {
                error.append(cleanup);
            }
            return Err(error);
        }
        let mut index = self.held.len();
        while index > 0 {
            index -= 1;
            if let Err(error) = transaction::verify(&self.held[index..=index], &self.devices) {
                self.remove_failed_reader(index, error, out)?;
            }
        }
        if self.scan_generation != previous_scan {
            self.open_readers();
        }
        let mut reads = stream::READ_BUDGET;
        let mut records = stream::EVENT_BUDGET;
        let mut snapshot = true;
        let mut idle = 0;
        while reads > 0 && records > 0 && idle < self.held.len() {
            let index = self.stream_cursor % self.held.len();
            self.stream_cursor = (index + 1) % self.held.len();
            let held = &mut self.held[index];
            if held.stream.is_none() {
                held.stream = Some(
                    stream::Stream::new(&held.info, self.monitor.now())
                        .map_err(|e| CaptureError::device("initialize stream", &held.info, e))?,
                );
            }
            match stream::advance(
                held,
                self.monitor.now(),
                &mut reads,
                &mut records,
                &mut snapshot,
                out,
            ) {
                Ok(progress) => idle = if progress { 0 } else { idle + 1 },
                Err(error) => {
                    let error = CaptureError::device("poll stream", &self.held[index].info, error);
                    self.remove_failed_reader(index, error, out)?;
                }
            }
        }
        self.monitor_status(out)
    }

    fn monitor_status(
        &mut self,
        out: &mut Vec<StreamUpdate>,
    ) -> Result<StreamStatus, CaptureError> {
        let mut status = StreamStatus {
            ready: true,
            pending: false,
        };
        let mut index = self.held.len();
        while index > 0 {
            index -= 1;
            let held = &self.held[index];
            let Some(stream) = &held.stream else {
                status.pending = true;
                continue;
            };
            let health =
                transaction::verify(std::slice::from_ref(held), &self.devices).and_then(|()| {
                    stream
                        .check_deadline(self.monitor.now())
                        .and_then(|()| held.handle.stream_pending())
                        .map_err(|e| CaptureError::device("poll stream descriptor", &held.info, e))
                });
            match health {
                Ok(pending) => {
                    if self.ids.is_empty() || held.grabbed {
                        status.ready &= stream.ready();
                    }
                    status.pending |= stream.pending() || pending;
                }
                Err(error) => self.remove_failed_reader(index, error, out)?,
            }
        }
        Ok(status)
    }
    fn remove_failed_reader(
        &mut self,
        index: usize,
        mut error: CaptureError,
        out: &mut Vec<StreamUpdate>,
    ) -> Result<(), CaptureError> {
        let mut held = self.held.remove(index);
        out.push(StreamUpdate::Reset {
            device: held.info.identity.id.clone().expect("monitored identity"),
        });
        if held.grabbed {
            if let Err(cleanup) = held.ungrab() {
                error.append(cleanup);
            }
            drop(held);
            if let Err(cleanup) = self.release() {
                error.append(cleanup);
            }
            return Err(error);
        }
        Ok(())
    }
}
