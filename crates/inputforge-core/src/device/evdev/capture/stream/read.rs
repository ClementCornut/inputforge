use super::{
    super::{
        handle::Handle,
        hotplug::{check_health, readiness},
    },
    NativeState,
};
use crate::device::evdev::Device;
use evdev::InputEvent;
use nix::poll::PollFlags;
use std::{collections::VecDeque, io};

// A dependency batch is normally 32 records; cap unexpected growth instead of losing tails.
const BATCH_LIMIT: usize = 256;

impl Handle {
    pub(in super::super) fn read_events(&mut self) -> io::Result<VecDeque<InputEvent>> {
        let mut out = VecDeque::new();
        match self {
            Self::Native { device, .. } => {
                // The drain must be exhausted on success. An oversize batch is fatal.
                for event in device.fetch_events()? {
                    if out.len() == BATCH_LIMIT {
                        return Err(oversized());
                    }
                    out.push_back(event);
                }
            }
            #[cfg(test)]
            Self::Fake(handle) => {
                use super::super::tests::fixtures::operation;
                operation(&handle.world, "read events", &handle.name)?;
                let mut world = handle.world.borrow_mut();
                let stream = world.streams.entry(handle.name.clone()).or_default();
                stream.reads_count += 1;
                let batch = stream
                    .reads
                    .pop_front()
                    .unwrap_or(Err(11))
                    .map_err(io::Error::from_raw_os_error)?;
                if batch.len() > BATCH_LIMIT {
                    return Err(oversized());
                }
                out.extend(batch);
            }
        }
        if out.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "event descriptor reached EOF",
            ));
        }
        Ok(out)
    }

    pub(in super::super) fn stream_pending(&self) -> io::Result<bool> {
        match self {
            Self::Native { device, .. } => {
                let flags = readiness(device.as_ref(), PollFlags::POLLIN)?;
                check_health(flags)?;
                Ok(flags.contains(PollFlags::POLLIN))
            }
            #[cfg(test)]
            Self::Fake(handle) => {
                super::super::tests::fixtures::operation(
                    &handle.world,
                    "poll stream descriptor",
                    &handle.name,
                )?;
                Ok(handle
                    .world
                    .borrow()
                    .streams
                    .get(&handle.name)
                    .is_some_and(|s| !s.reads.is_empty()))
            }
        }
    }

    pub(in super::super) fn snapshot(&self, info: &Device) -> io::Result<NativeState> {
        match self {
            Self::Native { device, .. } => {
                let mut state = NativeState::default();
                if !info.metadata.keys.is_empty() {
                    let keys = device.get_key_state()?;
                    state.keys = info
                        .metadata
                        .keys
                        .iter()
                        .map(|&code| (code, keys.contains(evdev::KeyCode(code))))
                        .collect();
                }
                if !info.metadata.abs_axes.is_empty() {
                    for (code, value) in device.get_absinfo()? {
                        if !info.axes.iter().any(|axis| {
                            axis.code == code.0
                                && axis.minimum == value.minimum()
                                && axis.maximum == value.maximum()
                                && axis.flat == value.flat()
                                && axis.fuzz == value.fuzz()
                                && axis.resolution == value.resolution()
                        }) {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "ABS metadata changed during streaming",
                            ));
                        }
                        state.axes.insert(code.0, value.value());
                    }
                }
                Ok(state)
            }
            #[cfg(test)]
            Self::Fake(handle) => {
                super::super::tests::fixtures::operation(
                    &handle.world,
                    "query stream state",
                    &handle.name,
                )?;
                let mut world = handle.world.borrow_mut();
                let stream = world.streams.entry(handle.name.clone()).or_default();
                stream.snapshots += 1;
                if !stream.during_snapshot.is_empty() {
                    let batch = std::mem::take(&mut stream.during_snapshot);
                    stream.reads.push_back(Ok(batch));
                }
                Ok(stream.state.clone())
            }
        }
    }
}

fn oversized() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "event batch exceeds 256 records",
    )
}
