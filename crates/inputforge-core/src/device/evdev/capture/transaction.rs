use super::{Capture, CaptureError, handle::Handle};
use crate::{
    device::evdev::{Access, Class, Device, IdentityQuality},
    types::DeviceId,
};
use std::io;

#[derive(Debug)]
pub(super) struct Held {
    pub info: Device,
    pub handle: Handle,
    pub(super) grabbed: bool,
    pub stream: Option<super::stream::Stream>,
}

impl Held {
    pub(super) fn ungrab(&mut self) -> Result<(), CaptureError> {
        if std::mem::take(&mut self.grabbed) {
            self.handle
                .ungrab()
                .map_err(|e| CaptureError::device("ungrab", &self.info, e))?;
        }
        Ok(())
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // Closing releases the grab even if the ioctl fails. Never log here: other
        // selected descriptors may still be held. Explicit release collects errors.
        let _ = self.ungrab();
    }
}

pub(super) fn selected<'a>(
    devices: &'a [Device],
    id: &DeviceId,
) -> Result<&'a Device, CaptureError> {
    let mut matches = devices
        .iter()
        .filter(|d| d.identity.id.as_ref() == Some(id));
    let device = matches.next().ok_or_else(|| {
        CaptureError::io(
            "select controller",
            std::path::Path::new(""),
            Some(id),
            io::Error::new(
                io::ErrorKind::NotFound,
                "controller identity is absent or ambiguous",
            ),
        )
    })?;
    if !id.0.starts_with("evdev:v1:")
        || matches.next().is_some()
        || device.classification.kind != Class::Controller
        || device.identity.quality == IdentityQuality::Ambiguous
    {
        return Err(CaptureError::device(
            "select controller",
            device,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "controller is not uniquely eligible and readable",
            ),
        ));
    }
    if device.access != Access::Readable {
        if let Some(issue) = device.issues.last() {
            return Err(CaptureError::from_issue(issue.clone(), device));
        }
        return Err(CaptureError::device(
            "select controller",
            device,
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "controller metadata is not readable",
            ),
        ));
    }
    Ok(device)
}

pub(super) fn verify(held: &[Held], devices: &[Device]) -> Result<(), CaptureError> {
    for held in held {
        held.handle
            .health()
            .map_err(|e| CaptureError::device("poll capture descriptor", &held.info, e))?;
        let id = held.info.identity.id.as_ref().expect("selected identity");
        let current = selected(devices, id)?;
        if held.info.metadata != current.metadata || held.info.axes != current.axes {
            return Err(CaptureError::device(
                "verify captured inventory",
                &held.info,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "selected interface metadata changed",
                ),
            ));
        }
        held.handle
            .verify_path(current)
            .map_err(|e| CaptureError::device("verify event node", current, e))?;
    }
    Ok(())
}

pub(super) fn release(held: &mut Vec<Held>) -> Result<(), CaptureError> {
    let mut error: Option<CaptureError> = None;
    for mut device in held.drain(..).rev() {
        if let Err(next) = device.ungrab() {
            if let Some(first) = &mut error {
                first.append(next);
            } else {
                error = Some(next);
            }
        }
        // Drop every descriptor, even when an explicit ungrab failed.
    }
    error.map_or(Ok(()), Err)
}

impl Capture {
    pub(super) fn acquire_set(&mut self, ids: &[DeviceId]) -> Result<(), CaptureError> {
        self.ensure_valid()?;
        if !self.held.is_empty() {
            return Err(CaptureError::state(
                "acquire",
                "release the current selection first",
            ));
        }
        if ids.is_empty() {
            return Err(CaptureError::state(
                "select controller",
                "select at least one controller",
            ));
        }
        let mut ids = ids.to_vec();
        ids.sort_by(|a, b| a.0.cmp(&b.0));
        ids.dedup();
        self.refresh(true)?;
        self.ensure_settled()?;
        let devices = ids
            .iter()
            .map(|id| selected(&self.devices, id).cloned())
            .collect::<Result<Vec<_>, _>>()?;
        let mut pending = Vec::new();
        let result = (|| {
            for info in devices {
                let handle = Handle::open(&self.monitor, &info)?;
                pending.push(Held {
                    info,
                    handle,
                    grabbed: false,
                    stream: None,
                });
            }
            for held in &mut pending {
                held.handle
                    .verify_path(&held.info)
                    .map_err(|e| CaptureError::device("verify event node", &held.info, e))?;
                held.handle
                    .grab()
                    .map_err(|e| CaptureError::device("grab", &held.info, e))?;
                held.grabbed = true;
            }
            self.refresh(true)?;
            self.ensure_settled()?;
            verify(&pending, &self.devices)
        })();
        if let Err(mut error) = result {
            if let Err(cleanup) = release(&mut pending) {
                error.append(cleanup);
            }
            return Err(error);
        }
        self.held = pending;
        self.ids = ids;
        Ok(())
    }
}
