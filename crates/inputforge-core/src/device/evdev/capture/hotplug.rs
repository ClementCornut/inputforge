use std::{
    fmt, io,
    os::fd::AsFd,
    time::{Duration, Instant},
};

use crate::device::evdev::{Device, discovery};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

// Bound event bursts so callers retain control of stop/deadline handling.
const EVENT_BUDGET: usize = 256;
// Recover inventory changes lost by libudev without scanning at input-frame frequency.
pub(super) const RECONCILE_INTERVAL: Duration = Duration::from_secs(1);

pub(super) enum Monitor {
    Native(udev::MonitorSocket),
    #[cfg(test)]
    Fake(super::tests::fixtures::Fake),
}

impl fmt::Debug for Monitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Monitor").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(super) struct Notifications {
    pub changed: bool,
    pub backlog: bool,
}

pub(super) fn readiness(fd: impl AsFd, interests: PollFlags) -> io::Result<PollFlags> {
    let mut fds = [PollFd::new(fd.as_fd(), interests)];
    match poll(&mut fds, PollTimeout::ZERO) {
        Ok(_) => Ok(fds[0].revents().unwrap_or_else(PollFlags::empty)),
        Err(error) => Err(io::Error::from_raw_os_error(error as i32)),
    }
}

pub(super) fn check_health(flags: PollFlags) -> io::Result<()> {
    if flags.intersects(PollFlags::POLLERR | PollFlags::POLLHUP | PollFlags::POLLNVAL) {
        Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "descriptor disconnected or revoked",
        ))
    } else {
        Ok(())
    }
}

impl Monitor {
    pub(super) fn has_pending(&self) -> io::Result<bool> {
        match self {
            Self::Native(socket) => {
                let flags = readiness(socket, PollFlags::POLLIN)?;
                check_health(flags)?;
                Ok(flags.contains(PollFlags::POLLIN))
            }
            #[cfg(test)]
            Self::Fake(fake) => Ok(fake.borrow().pending != 0),
        }
    }

    pub(super) fn new() -> io::Result<Self> {
        // Subscribe to processed udev events before Capture performs its first scan.
        Ok(Self::Native(
            udev::MonitorBuilder::new()?
                .match_subsystem("input")?
                .listen()?,
        ))
    }

    pub(super) fn now(&self) -> Instant {
        match self {
            Self::Native(_) => Instant::now(),
            #[cfg(test)]
            Self::Fake(fake) => fake.borrow().now,
        }
    }

    pub(super) fn scan(&mut self) -> io::Result<Vec<Device>> {
        match self {
            Self::Native(_) => discovery::scan(),
            #[cfg(test)]
            Self::Fake(fake) => {
                let mut world = fake.borrow_mut();
                world.scans += 1;
                if world.scan_error {
                    return Err(io::Error::from_raw_os_error(5));
                }
                if world
                    .replace_on_scan
                    .as_ref()
                    .is_some_and(|(scan, _)| *scan == world.scans)
                {
                    world.devices = world.replace_on_scan.take().expect("matched scan").1;
                }
                if world
                    .pending_on_scan
                    .as_ref()
                    .is_some_and(|(scan, _)| *scan == world.scans)
                {
                    world.pending += world.pending_on_scan.take().expect("matched scan").1;
                }
                Ok(world.devices.clone())
            }
        }
    }

    pub(super) fn events(&mut self) -> io::Result<Notifications> {
        match self {
            Self::Native(socket) => {
                check_health(readiness(&*socket, PollFlags::POLLIN)?)?;
                let mut changed = false;
                for event in socket.iter().take(EVENT_BUDGET) {
                    // Parent input changes can affect event classification or identity too.
                    changed |= matches!(
                        event.event_type(),
                        udev::EventType::Add
                            | udev::EventType::Remove
                            | udev::EventType::Change
                            | udev::EventType::Bind
                            | udev::EventType::Unbind
                    );
                }
                let flags = readiness(&*socket, PollFlags::POLLIN)?;
                check_health(flags)?;
                Ok(Notifications {
                    changed,
                    backlog: flags.contains(PollFlags::POLLIN),
                })
            }
            #[cfg(test)]
            Self::Fake(fake) => {
                let mut world = fake.borrow_mut();
                if world.monitor_error {
                    return Err(io::Error::from_raw_os_error(5));
                }
                let changed = world.pending != 0;
                world.pending = world.pending.saturating_sub(EVENT_BUDGET);
                Ok(Notifications {
                    changed,
                    backlog: world.pending != 0,
                })
            }
        }
    }
}
