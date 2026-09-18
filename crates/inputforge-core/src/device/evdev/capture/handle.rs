use evdev::raw_stream::RawDevice;
use nix::poll::PollFlags;
use std::{
    fs::{self, OpenOptions},
    io,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
};

use super::{
    CaptureError,
    hotplug::{Monitor, check_health, readiness},
};
use crate::device::evdev::{Device, probe};

#[derive(Debug)]
pub(super) enum Handle {
    Native {
        device: Box<RawDevice>,
        stamp: (u64, u64, u64),
    },
    #[cfg(test)]
    Fake(super::tests::fixtures::FakeHandle),
}

fn stamp(metadata: &fs::Metadata) -> (u64, u64, u64) {
    (metadata.dev(), metadata.ino(), metadata.rdev())
}

impl Handle {
    pub(super) fn open(monitor: &Monitor, info: &Device) -> Result<Self, CaptureError> {
        match monitor {
            Monitor::Native(_) => {
                // Never use RawDevice::open: it attempts O_RDWR before O_RDONLY.
                let file = OpenOptions::new()
                    .read(true)
                    .custom_flags(nix::libc::O_NONBLOCK)
                    .open(&info.metadata.node)
                    .map_err(|e| CaptureError::device("open read-only", info, e))?;
                let stamp = stamp(
                    &file
                        .metadata()
                        .map_err(|e| CaptureError::device("stat capture descriptor", info, e))?,
                );
                let device = RawDevice::from_fd(file.into())
                    .map_err(|e| CaptureError::device("query evdev metadata", info, e))?;
                probe::verify(&info.metadata, &device)
                    .map_err(|issue| CaptureError::from_issue(issue, info))?;
                Ok(Self::Native {
                    device: Box::new(device),
                    stamp,
                })
            }
            #[cfg(test)]
            Monitor::Fake(fake) => {
                use super::tests::fixtures::{FakeHandle, operation};
                let name = &info.metadata.name;
                operation(fake, "open read-only", name)
                    .map_err(|e| CaptureError::device("open read-only", info, e))?;
                fake.borrow_mut().opened.insert(name.clone());
                let handle = Self::Fake(FakeHandle {
                    world: std::rc::Rc::clone(fake),
                    name: name.clone(),
                    generation: fake.borrow().generation,
                });
                operation(fake, "verify event node", name)
                    .map_err(|e| CaptureError::device("verify event node", info, e))?;
                Ok(handle)
            }
        }
    }

    pub(super) fn grab(&mut self) -> io::Result<()> {
        match self {
            Self::Native { device, .. } => device.grab(),
            #[cfg(test)]
            Self::Fake(handle) => {
                super::tests::fixtures::operation(&handle.world, "grab", &handle.name)?;
                handle
                    .world
                    .borrow_mut()
                    .grabbed
                    .insert(handle.name.clone());
                Ok(())
            }
        }
    }

    pub(super) fn ungrab(&mut self) -> io::Result<()> {
        match self {
            Self::Native { device, .. } => device.ungrab(),
            #[cfg(test)]
            Self::Fake(handle) => {
                super::tests::fixtures::operation(&handle.world, "ungrab", &handle.name)?;
                handle.world.borrow_mut().grabbed.remove(&handle.name);
                Ok(())
            }
        }
    }

    pub(super) fn health(&self) -> io::Result<()> {
        match self {
            // An empty interest mask avoids unread input continually waking the caller.
            Self::Native { device, .. } => {
                check_health(readiness(device.as_ref(), PollFlags::empty())?)
            }
            #[cfg(test)]
            Self::Fake(handle) => {
                if handle.world.borrow().dead.contains(&handle.name) {
                    Err(io::Error::from_raw_os_error(19))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub(super) fn verify_path(&self, info: &Device) -> io::Result<()> {
        let same = match self {
            Self::Native {
                stamp: expected, ..
            } => *expected == stamp(&fs::metadata(&info.metadata.node)?),
            #[cfg(test)]
            Self::Fake(handle) => handle.generation == handle.world.borrow().generation,
        };
        if same {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "event node was replaced; reacquire explicitly",
            ))
        }
    }
}
