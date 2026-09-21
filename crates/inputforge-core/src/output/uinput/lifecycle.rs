use super::{
    Error, Output,
    device::DeviceSpec,
    native::{Handle, System, write},
    state::State,
};
use std::{
    io,
    time::{Duration, Instant},
};

// Userspace deadlines, checked between synchronous syscalls; never hard realtime.
pub(super) const CREATE_LIMIT: Duration = Duration::from_secs(2);
pub(super) const WRITE_LIMIT: Duration = Duration::from_millis(50);
pub(super) const READY_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug)]
pub(super) struct Held {
    pub(super) slot: u8,
    handle: Handle,
    pub(super) state: State,
}

impl Held {
    fn flush(&mut self, system: &System, deadline: Instant, force: bool) -> Result<(), Error> {
        for packet in self.state.packets(force) {
            write::packet(
                &packet,
                |events| self.handle.write(events),
                || system.now(),
                deadline,
            )
            .map_err(|e| Error::io("write output packet", Some(self.slot), "/dev/uinput", e))?;
        }
        self.state.commit();
        Ok(())
    }
}

impl Output {
    pub(super) fn create_all(&mut self) -> Result<(), Error> {
        self.ensure_valid()?;
        if self.is_active() {
            return Err(Error::invalid("create", None, "output is already active"));
        }
        let result = self.create_inner();
        if let Err(mut error) = result {
            // Nothing but neutral state has been submitted during creation.
            if let Err(cleanup) = self.destroy_all() {
                error.append(cleanup);
            }
            return Err(error);
        }
        Ok(())
    }

    fn create_inner(&mut self) -> Result<(), Error> {
        let deadline = self.system.now() + CREATE_LIMIT;
        for cfg in &self.configs {
            check_deadline(&self.system, deadline, "create", Some(cfg.device_id))?;
            let handle = self.system.create(&DeviceSpec::controller(cfg))?;
            self.held.push(Held {
                slot: cfg.device_id,
                handle,
                state: State::new(cfg),
            });
        }
        for (held, cfg) in self.held.iter_mut().zip(&self.configs) {
            let spec = DeviceSpec::controller(cfg);
            loop {
                check_deadline(&self.system, deadline, "await event node", Some(held.slot))?;
                match held.handle.ready(&spec) {
                    Ok(()) => break,
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::NotFound
                                | io::ErrorKind::PermissionDenied
                                | io::ErrorKind::WouldBlock
                        ) =>
                    {
                        if self.system.now() + READY_INTERVAL >= deadline {
                            return Err(Error::io(
                                "event node readiness deadline",
                                Some(held.slot),
                                "/dev/input",
                                error,
                            ));
                        }
                        self.system.sleep(READY_INTERVAL);
                    }
                    Err(error) => {
                        return Err(Error::io(
                            "verify event node",
                            Some(held.slot),
                            "/dev/input",
                            error,
                        ));
                    }
                }
            }
        }
        check_deadline(&self.system, deadline, "initialize output", None)?;
        let write_deadline = deadline.min(self.system.now() + WRITE_LIMIT);
        for held in &mut self.held {
            held.flush(&self.system, write_deadline, true)?;
        }
        check_deadline(&self.system, deadline, "create", None)
    }

    pub(super) fn flush_all(&mut self, force: bool) -> Result<(), Error> {
        self.ensure_active()?;
        match self.flush_deferred(force) {
            Ok(()) => Ok(()),
            Err((failed, mut error)) => {
                if let Err(cleanup) = self.release_all(Some(failed)) {
                    error.append(cleanup);
                }
                Err(error)
            }
        }
    }

    // Caller checks active state. No output cleanup occurs on this engine-only path.
    pub(super) fn flush_deferred(&mut self, force: bool) -> Result<(), (u8, Error)> {
        let deadline = self.system.now() + WRITE_LIMIT;
        for held in &mut self.held {
            if let Err(error) = held.flush(&self.system, deadline, force) {
                self.valid = false;
                return Err((held.slot, error));
            }
        }
        Ok(())
    }

    pub(super) fn release_all(&mut self, skip_neutral: Option<u8>) -> Result<(), Error> {
        let deadline = self.system.now() + WRITE_LIMIT;
        let mut failure = None;
        for mut held in self.held.drain(..) {
            if skip_neutral != Some(held.slot) {
                held.state.neutral();
                let result = held.flush(&self.system, deadline, true);
                self.valid &= result.is_ok();
                collect(&mut failure, result);
            }
            collect(
                &mut failure,
                held.handle
                    .destroy()
                    .map_err(|e| Error::io("destroy output", Some(held.slot), "/dev/uinput", e)),
            );
        }
        failure.map_or(Ok(()), Err)
    }

    fn destroy_all(&mut self) -> Result<(), Error> {
        let mut failure = None;
        for mut held in self.held.drain(..) {
            collect(
                &mut failure,
                held.handle
                    .destroy()
                    .map_err(|e| Error::io("rollback output", Some(held.slot), "/dev/uinput", e)),
            );
        }
        failure.map_or(Ok(()), Err)
    }
}

pub(super) fn check_deadline(
    system: &System,
    deadline: Instant,
    operation: &'static str,
    slot: Option<u8>,
) -> Result<(), Error> {
    if system.now() < deadline {
        Ok(())
    } else {
        Err(Error::io(
            operation,
            slot,
            "/dev/uinput",
            io::Error::new(io::ErrorKind::TimedOut, "creation deadline"),
        ))
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
