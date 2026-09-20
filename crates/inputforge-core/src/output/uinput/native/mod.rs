mod readiness;
mod setup;
pub(super) mod write;

use super::Error;
use crate::types::VirtualDeviceConfig;
use evdev::InputEvent;
use std::{
    io,
    os::fd::{AsRawFd, OwnedFd},
    time::{Duration, Instant},
};

/// Native operations or a scripted syscall boundary, never a selectable production mock.
#[derive(Debug)]
pub(super) enum System {
    Native,
    #[cfg(test)]
    Fake(super::tests::fixtures::World),
}

impl System {
    pub(super) fn now(&self) -> Instant {
        match self {
            Self::Native => Instant::now(),
            #[cfg(test)]
            Self::Fake(world) => world.lock().unwrap().now,
        }
    }
    pub(super) fn sleep(&self, duration: Duration) {
        match self {
            Self::Native => std::thread::sleep(duration),
            #[cfg(test)]
            Self::Fake(world) => world.lock().unwrap().now += duration,
        }
    }
    pub(super) fn create(&self, config: &VirtualDeviceConfig) -> Result<Handle, Error> {
        match self {
            Self::Native => setup::create(config),
            #[cfg(test)]
            Self::Fake(world) => super::tests::fixtures::create(world, config).map(Handle::Fake),
        }
    }
}

#[derive(Debug)]
pub(super) enum Handle {
    Native(Option<OwnedFd>),
    #[cfg(test)]
    Fake(super::tests::fixtures::FakeHandle),
}

impl Handle {
    pub(super) fn ready(&mut self, config: &VirtualDeviceConfig) -> io::Result<()> {
        match self {
            Self::Native(Some(device)) => readiness::check(device, config),
            Self::Native(None) => Err(io::ErrorKind::NotConnected.into()),
            #[cfg(test)]
            Self::Fake(handle) => handle.ready(),
        }
    }

    #[expect(
        unsafe_code,
        reason = "write initialized evdev repr(transparent) native input_event records to an owned descriptor"
    )]
    pub(super) fn write(&mut self, events: &[InputEvent]) -> io::Result<usize> {
        match self {
            Self::Native(Some(device)) => {
                // SAFETY: evdev InputEvent is transparent over the Linux input_event ABI.
                // InputEvent::new initializes its timeval and every scalar field. The slice
                // remains alive for the call; libc only reads it and borrows the live fd.
                let result = unsafe {
                    nix::libc::write(
                        device.as_raw_fd(),
                        events.as_ptr().cast(),
                        size_of_val(events),
                    )
                };
                nix::errno::Errno::result(result)
                    .map(|bytes| usize::try_from(bytes).expect("nonnegative write count"))
                    .map_err(io::Error::from)
            }
            Self::Native(None) => Err(io::ErrorKind::NotConnected.into()),
            #[cfg(test)]
            Self::Fake(handle) => handle.write(events),
        }
    }

    #[expect(
        unsafe_code,
        reason = "UI_DEV_DESTROY has no pointer arguments and borrows the owned live fd"
    )]
    pub(super) fn destroy(&mut self) -> io::Result<()> {
        match self {
            Self::Native(device) => {
                let Some(device) = device.take() else {
                    return Ok(());
                };
                // SAFETY: the ioctl takes no arguments; device owns the descriptor until
                // after the call. Taking it first guarantees close even on ioctl failure.
                let result = unsafe {
                    nix::libc::ioctl(device.as_raw_fd(), nix::request_code_none!(b'U', 2))
                };
                nix::errno::Errno::result(result)
                    .map(|_| ())
                    .map_err(io::Error::from)
            }
            #[cfg(test)]
            Self::Fake(handle) => handle.destroy(),
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _result = self.destroy();
    }
}
