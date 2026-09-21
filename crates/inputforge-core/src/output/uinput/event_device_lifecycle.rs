use super::{
    Error,
    device::DeviceKey,
    event_device::{EventDevice, EventError},
    lifecycle::{CREATE_LIMIT, READY_INTERVAL, WRITE_LIMIT, check_deadline},
    native::write,
};
use crate::output::OutputPhase;
use evdev::InputEvent;
use std::{io, time::Instant};

impl EventDevice {
    pub(super) fn start(&mut self) -> Result<(), EventError> {
        if self.handle.is_some() && self.valid {
            return Ok(());
        }
        if !self.valid {
            return Err(self.invalid(OutputPhase::Initialization, "device requires cleanup"));
        }
        let deadline = self.system.now() + CREATE_LIMIT;
        let handle = self.system.create(&self.spec).map_err(|error| {
            self.valid = false;
            EventError {
                phase: OutputPhase::Initialization,
                error,
            }
        })?;
        self.handle = Some(handle);
        if let Err(error) = self.await_ready(deadline) {
            self.valid = false;
            return Err(error);
        }
        let packet = [InputEvent::new(0, 0, 0)];
        if let Err(error) = self.write(&packet, deadline.min(self.system.now() + WRITE_LIMIT)) {
            self.valid = false;
            return Err(EventError {
                phase: OutputPhase::Initialization,
                error,
            });
        }
        Ok(())
    }

    pub(super) fn stop(&mut self) -> Result<(), EventError> {
        let deadline = self.system.now() + WRITE_LIMIT;
        let mut failure = None;
        if self.handle.is_some() {
            for code in std::mem::take(&mut self.possible) {
                let packet = [InputEvent::new(1, code, 0), InputEvent::new(0, 0, 0)];
                collect(&mut failure, self.write(&packet, deadline));
            }
            let result = self
                .handle
                .as_mut()
                .expect("checked handle")
                .destroy()
                .map_err(|error| {
                    Error::io("destroy output", self.spec.key.slot(), "/dev/uinput", error)
                });
            collect(&mut failure, result);
        }
        self.handle = None;
        self.groups.clear();
        self.claims.clear();
        self.possible.clear();
        self.valid = true;
        failure.map_or(Ok(()), |error| {
            Err(EventError {
                phase: OutputPhase::Release,
                error,
            })
        })
    }

    fn await_ready(&mut self, deadline: Instant) -> Result<(), EventError> {
        loop {
            check_deadline(
                &self.system,
                deadline,
                "await output readiness",
                self.spec.key.slot(),
            )
            .map_err(|error| EventError {
                phase: OutputPhase::Readiness,
                error,
            })?;
            match self
                .handle
                .as_mut()
                .expect("created handle")
                .ready(&self.spec)
            {
                Ok(()) => return Ok(()),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound
                            | io::ErrorKind::PermissionDenied
                            | io::ErrorKind::WouldBlock
                    ) =>
                {
                    if self.system.now() + READY_INTERVAL >= deadline {
                        return Err(EventError {
                            phase: OutputPhase::Readiness,
                            error: Error::io(
                                "output readiness deadline",
                                self.spec.key.slot(),
                                readiness_path(self.spec.key),
                                error,
                            ),
                        });
                    }
                    self.system.sleep(READY_INTERVAL);
                }
                Err(error) => {
                    return Err(EventError {
                        phase: OutputPhase::Readiness,
                        error: Error::io(
                            "verify output readiness",
                            self.spec.key.slot(),
                            readiness_path(self.spec.key),
                            error,
                        ),
                    });
                }
            }
        }
    }

    pub(super) fn write(&mut self, packet: &[InputEvent], deadline: Instant) -> Result<(), Error> {
        let key = self.spec.key;
        let system = &self.system;
        let handle = self.handle.as_mut().ok_or_else(|| {
            Error::invalid("write event packet", key.slot(), "output is inactive")
        })?;
        write::packet(
            packet,
            |events| handle.write(events),
            || system.now(),
            deadline,
        )
        .map_err(|error| Error::io("write event packet", key.slot(), "/dev/uinput", error))
    }
}

const fn readiness_path(key: DeviceKey) -> &'static str {
    match key {
        DeviceKey::Controller(_) => "/dev/input",
        DeviceKey::Keyboard | DeviceKey::Mouse => "/sys/devices/virtual/input",
    }
}

fn collect(failure: &mut Option<Error>, result: Result<(), Error>) {
    if let Err(error) = result {
        if let Some(primary) = failure {
            primary.append(error);
        } else {
            *failure = Some(error);
        }
    }
}
