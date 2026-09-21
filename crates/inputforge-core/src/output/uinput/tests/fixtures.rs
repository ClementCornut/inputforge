use super::super::{
    Error, Output,
    device::{DeviceKey, DeviceSpec},
    native::System,
};
use crate::output::MouseSink;
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
    pub alive: BTreeSet<DeviceKey>,
    pub calls: Vec<(DeviceKey, &'static str)>,
    pub emitted: Vec<(DeviceKey, Vec<InputEvent>)>,
    pub fail_create: Option<DeviceKey>,
    pub fail_ready: Option<(DeviceKey, io::ErrorKind)>,
    pub fail_destroy: BTreeSet<DeviceKey>,
    pub writes: VecDeque<io::Result<usize>>,
    pub write_delay: Duration,
}

pub(super) fn output(configs: Vec<VirtualDeviceConfig>) -> (Output, World) {
    let world = world();
    let mut output = Output::new(configs).unwrap();
    output.system = System::Fake(Arc::clone(&world));
    (output, world)
}

fn world() -> World {
    Arc::new(Mutex::new(Script {
        now: Instant::now(),
        alive: BTreeSet::new(),
        calls: vec![],
        emitted: vec![],
        fail_create: None,
        fail_ready: None,
        fail_destroy: BTreeSet::new(),
        writes: VecDeque::new(),
        write_delay: Duration::ZERO,
    }))
}

pub(super) fn event_device(keys: Vec<u16>) -> (super::super::event_device::EventDevice, World) {
    let world = world();
    let device = super::super::event_device::EventDevice::with_system(
        DeviceSpec::keyboard(keys),
        System::Fake(Arc::clone(&world)),
    );
    (device, world)
}

pub(super) fn started_event_device(
    keys: Vec<u16>,
) -> (super::super::event_device::EventDevice, World) {
    let (mut device, world) = event_device(keys);
    device.start().unwrap();
    world.lock().unwrap().emitted.clear();
    (device, world)
}

pub(super) fn keyboard() -> (super::super::Keyboard, World) {
    let world = world();
    let keyboard = super::super::Keyboard {
        device: super::super::event_device::EventDevice::with_system(
            DeviceSpec::keyboard(super::super::key_codes::all_codes()),
            System::Fake(Arc::clone(&world)),
        ),
    };
    (keyboard, world)
}

pub(super) fn started_mouse() -> (super::super::Mouse, World) {
    let world = world();
    let mut mouse = super::super::Mouse {
        device: super::super::event_device::EventDevice::with_system(
            DeviceSpec::mouse(),
            System::Fake(Arc::clone(&world)),
        ),
    };
    mouse.start().unwrap();
    world.lock().unwrap().emitted.clear();
    (mouse, world)
}

pub(in crate::output::uinput) fn create(
    world: &World,
    spec: &DeviceSpec,
) -> Result<FakeHandle, Error> {
    let key = spec.key;
    let mut script = world.lock().unwrap();
    script.calls.push((key, "create"));
    if script.fail_create == Some(key) {
        return Err(Error::io(
            "create uinput",
            key.slot(),
            "/dev/uinput",
            io::Error::from_raw_os_error(13),
        ));
    }
    script.alive.insert(key);
    Ok(FakeHandle {
        world: Arc::clone(world),
        key,
        closed: false,
    })
}

#[derive(Debug)]
pub(in crate::output::uinput) struct FakeHandle {
    world: World,
    key: DeviceKey,
    closed: bool,
}
impl FakeHandle {
    pub(in crate::output::uinput) fn ready(&self) -> io::Result<()> {
        let mut script = self.world.lock().unwrap();
        script.calls.push((self.key, "ready"));
        if let Some((key, kind)) = script.fail_ready
            && key == self.key
        {
            return Err(kind.into());
        }
        Ok(())
    }
    pub(in crate::output::uinput) fn write(&self, events: &[InputEvent]) -> io::Result<usize> {
        let mut script = self.world.lock().unwrap();
        script.calls.push((self.key, "write"));
        let delay = script.write_delay;
        script.now += delay;
        let result = script.writes.pop_front().unwrap_or(Ok(size_of_val(events)));
        if let Ok(bytes) = result {
            script.emitted.push((
                self.key,
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
        script.calls.push((self.key, "destroy"));
        script.alive.remove(&self.key);
        script.calls.push((self.key, "close"));
        if script.fail_destroy.contains(&self.key) {
            return Err(io::Error::from_raw_os_error(5));
        }
        Ok(())
    }
}
