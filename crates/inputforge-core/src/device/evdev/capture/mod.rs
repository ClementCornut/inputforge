mod error;
mod handle;
mod hotplug;
mod passive;
mod passive_transaction;
mod stream;
#[cfg(test)]
mod tests;
mod transaction;
pub use stream::{
    NativeChange, NativeControl, NativeHat, NativeState, SnapshotKind, StreamStatus, StreamUpdate,
};

use crate::{device::evdev::Device, types::DeviceId};
pub use error::CaptureError;
use hotplug::{Monitor, RECONCILE_INTERVAL};
use std::{path::Path, time::Instant};
use transaction::Held;

/// Monitors Linux controllers and owns an explicitly selected set of exclusive grabs.
///
/// Call [`poll`](Self::poll) regularly to detect disconnects and reconcile inventory.
/// Only [`poll_stream`](Self::poll_stream) opts into event reads; no API creates outputs.
/// It is thread-affine through its udev monitor and never automatically reacquires.
/// A polling error releases capture and invalidates the owner; construct a new one.
#[derive(Debug)]
pub struct Capture {
    monitor: Monitor,
    devices: Vec<Device>,
    held: Vec<Held>,
    ids: Vec<DeviceId>,
    last_scan: Instant,
    pending: bool,
    valid: bool,
    stream_cursor: usize,
    monitoring: bool,
    scan_generation: u64,
}

impl Capture {
    /// Subscribe to hotplug and enumerate controllers without grabbing any device.
    ///
    /// # Errors
    /// Returns contextual monitor setup or enumeration errors.
    pub fn new() -> Result<Self, CaptureError> {
        let monitor = Monitor::new()
            .map_err(|e| CaptureError::io("subscribe udev", Path::new(""), None, e))?;
        Self::from_monitor(monitor)
    }

    fn from_monitor(monitor: Monitor) -> Result<Self, CaptureError> {
        let now = monitor.now();
        let mut capture = Self {
            monitor,
            devices: Vec::new(),
            held: Vec::new(),
            ids: Vec::new(),
            last_scan: now,
            pending: false,
            valid: true,
            stream_cursor: 0,
            monitoring: false,
            scan_generation: 0,
        };
        capture.refresh(true)?;
        Ok(capture)
    }

    /// Return the latest native discovery records, including ineligible interfaces.
    #[must_use]
    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    /// Return the stable identities held by the last successful acquisition.
    #[must_use]
    pub fn captured(&self) -> &[DeviceId] {
        &self.ids
    }

    /// Process bounded hotplug notifications and check all captured descriptors.
    ///
    /// Reconciles inventory every second even without notifications. No input events
    /// are read; descriptor polling requests only health, not event readability.
    ///
    /// # Errors
    /// Releases all grabs and invalidates this owner on monitor, enumeration,
    /// selected-device health or identity failures. A new owner is then required.
    pub fn poll(&mut self) -> Result<(), CaptureError> {
        let result = (|| {
            self.ensure_valid()?;
            self.refresh(false)?;
            transaction::verify(&self.held, &self.devices)
        })();
        if let Err(mut error) = result {
            self.valid = false;
            if let Err(cleanup) = self.release() {
                error.append(cleanup);
            }
            return Err(error);
        }
        Ok(())
    }

    /// Acquire every selected controller, rolling back the entire attempt on failure.
    ///
    /// IDs are deduplicated and sorted. Grabs are sequential kernel operations;
    /// success publishes the complete set only after final validation.
    ///
    /// # Errors
    /// Rejects empty, unavailable, ambiguous or ineligible selections, active or
    /// invalid owners, unsettled hotplug, and open/validation/grab failures.
    /// Acquisition errors leave no new grabs; fatal monitor/scan errors also
    /// invalidate the owner. An already active selection remains unchanged.
    pub fn acquire(&mut self, ids: impl AsRef<[DeviceId]>) -> Result<(), CaptureError> {
        if self.monitoring {
            self.acquire_monitored(ids.as_ref())
        } else {
            self.acquire_set(ids.as_ref())
        }
    }

    /// Release all grabs and close all capture descriptors, including after errors.
    ///
    /// Repeated calls are harmless. The monitor remains open until owner teardown.
    ///
    /// # Errors
    /// Reports ungrab failures after still closing every capture descriptor.
    pub fn release(&mut self) -> Result<(), CaptureError> {
        self.ids.clear();
        self.stream_cursor = 0;
        if self.monitoring && self.valid {
            self.release_grabs()
        } else {
            transaction::release(&mut self.held)
        }
    }

    fn ensure_valid(&self) -> Result<(), CaptureError> {
        if self.valid {
            Ok(())
        } else {
            Err(CaptureError::state(
                "capture lifecycle",
                "owner is invalid; construct a new Capture",
            ))
        }
    }

    fn ensure_settled(&self) -> Result<(), CaptureError> {
        if self.pending {
            Err(CaptureError::state(
                "reconcile hotplug",
                "notifications remain pending; poll before retrying",
            ))
        } else {
            Ok(())
        }
    }

    fn refresh(&mut self, force: bool) -> Result<(), CaptureError> {
        let result = (|| {
            let events = self
                .monitor
                .events()
                .map_err(|e| CaptureError::io("poll udev", Path::new(""), None, e))?;
            let now = self.monitor.now();
            let refresh = force
                || self.pending
                || events.changed
                || events.backlog
                || now.duration_since(self.last_scan) >= RECONCILE_INTERVAL;
            self.pending = events.backlog;
            if refresh {
                self.devices = self
                    .monitor
                    .scan()
                    .map_err(|e| CaptureError::io("enumerate input", Path::new(""), None, e))?;
                self.last_scan = now;
                self.scan_generation = self.scan_generation.wrapping_add(1);
                // A notification can arrive during enumeration. Do not commit a
                // grab transaction until a later scan reconciles that notification.
                self.pending |= self
                    .monitor
                    .has_pending()
                    .map_err(|e| CaptureError::io("poll udev", Path::new(""), None, e))?;
            }
            Ok(())
        })();
        if result.is_err() {
            self.valid = false;
        }
        result
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        if let Err(error) = self.release() {
            tracing::warn!(%error, "capture.release_on_drop_failed");
        }
    }
}
