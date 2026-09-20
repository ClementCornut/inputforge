use super::super::{Error, Output, native::System};
use crate::types::VirtualDeviceConfig;
use evdev::InputEvent;
use std::{
    collections::{BTreeSet, VecDeque},
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub(in crate::output::uinput) type World = Arc<Mutex<Script>>;

#[derive(Debug)]
pub(in crate::output::uinput) struct Script {
    pub now: Instant,
    pub alive: BTreeSet<u8>,
    pub calls: Vec<(u8, &'static str)>,
    pub emitted: Vec<(u8, Vec<InputEvent>)>,
    pub fail_create: Option<u8>,
    pub fail_ready: Option<(u8, io::ErrorKind)>,
    pub fail_destroy: BTreeSet<u8>,
    pub writes: VecDeque<io::Result<usize>>,
    pub write_delay: Duration,
}

pub(super) fn output(configs: Vec<VirtualDeviceConfig>) -> (Output, World) {
    let world = Arc::new(Mutex::new(Script {
        now: Instant::now(),
        alive: BTreeSet::new(),
        calls: vec![],
        emitted: vec![],
        fail_create: None,
        fail_ready: None,
        fail_destroy: BTreeSet::new(),
        writes: VecDeque::new(),
        write_delay: Duration::ZERO,
    }));
    let mut output = Output::new(configs).unwrap();
    output.system = System::Fake(Arc::clone(&world));
    (output, world)
}

pub(in crate::output::uinput) fn create(
    world: &World,
    config: &VirtualDeviceConfig,
) -> Result<FakeHandle, Error> {
    let slot = config.device_id;
    let mut script = world.lock().unwrap();
    script.calls.push((slot, "create"));
    if script.fail_create == Some(slot) {
        return Err(Error::io(
            "create uinput",
            Some(slot),
            "/dev/uinput",
            io::Error::from_raw_os_error(13),
        ));
    }
    script.alive.insert(slot);
    Ok(FakeHandle {
        world: Arc::clone(world),
        slot,
        closed: false,
    })
}

#[derive(Debug)]
pub(in crate::output::uinput) struct FakeHandle {
    world: World,
    slot: u8,
    closed: bool,
}
impl FakeHandle {
    pub(in crate::output::uinput) fn ready(&self) -> io::Result<()> {
        let mut script = self.world.lock().unwrap();
        script.calls.push((self.slot, "ready"));
        if let Some((slot, kind)) = script.fail_ready
            && slot == self.slot
        {
            return Err(kind.into());
        }
        Ok(())
    }
    pub(in crate::output::uinput) fn write(&self, events: &[InputEvent]) -> io::Result<usize> {
        let mut script = self.world.lock().unwrap();
        script.calls.push((self.slot, "write"));
        let delay = script.write_delay;
        script.now += delay;
        let result = script.writes.pop_front().unwrap_or(Ok(size_of_val(events)));
        if let Ok(bytes) = result {
            script.emitted.push((
                self.slot,
                events[..(bytes / size_of::<InputEvent>()).min(events.len())].to_vec(),
            ));
        }
        result
    }
    pub(in crate::output::uinput) fn destroy(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let mut script = self.world.lock().unwrap();
        script.calls.push((self.slot, "destroy"));
        script.alive.remove(&self.slot);
        script.calls.push((self.slot, "close"));
        if script.fail_destroy.contains(&self.slot) {
            return Err(io::Error::from_raw_os_error(5));
        }
        Ok(())
    }
}
