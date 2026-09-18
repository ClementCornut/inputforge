use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    io,
    rc::Rc,
    time::Instant,
};

use super::super::{Capture, hotplug::Monitor};
use crate::device::evdev::{
    Access, Class, Classification, Device, Identity, IdentityQuality, Metadata,
};
use crate::types::DeviceId;

#[derive(Debug)]
pub(in super::super) struct World {
    pub devices: Vec<Device>,
    pub streams: BTreeMap<String, super::stream::fixtures::StreamIo>,
    pub scans: usize,
    pub replace_on_scan: Option<(usize, Vec<Device>)>,
    pub pending_on_scan: Option<(usize, usize)>,
    pub scan_error: bool,
    pub monitor_error: bool,
    pub pending: usize,
    pub now: Instant,
    pub failures: BTreeMap<(String, String), i32>,
    pub dead: BTreeSet<String>,
    pub generation: u64,
    pub opened: BTreeSet<String>,
    pub grabbed: BTreeSet<String>,
    pub calls: Vec<String>,
}

pub(in super::super) type Fake = Rc<RefCell<World>>;

pub(in super::super) fn id(name: &str) -> DeviceId {
    DeviceId(format!("evdev:v1:{name}"))
}

pub(in super::super) fn device(name: &str) -> Device {
    Device {
        metadata: Metadata {
            node: format!("/fixture/{name}").into(),
            name: name.into(),
            ..Metadata::default()
        },
        classification: Classification {
            kind: Class::Controller,
            reasons: vec![],
        },
        identity: Identity {
            id: Some(id(name)),
            quality: IdentityQuality::Serial,
        },
        axes: vec![],
        access: Access::Readable,
        issues: vec![],
    }
}

pub(in super::super) fn world() -> Fake {
    Rc::new(RefCell::new(World {
        devices: vec![device("a"), device("b"), device("c")],
        streams: BTreeMap::new(),
        scans: 0,
        replace_on_scan: None,
        pending_on_scan: None,
        scan_error: false,
        monitor_error: false,
        pending: 0,
        now: Instant::now(),
        failures: BTreeMap::new(),
        dead: BTreeSet::new(),
        generation: 1,
        opened: BTreeSet::new(),
        grabbed: BTreeSet::new(),
        calls: vec![],
    }))
}

pub(in super::super) fn capture(fake: &Fake) -> Capture {
    Capture::from_monitor(Monitor::Fake(Rc::clone(fake))).expect("fixture inventory")
}

pub(in super::super) fn fail(fake: &Fake, operation: &str, name: &str) {
    fake.borrow_mut()
        .failures
        .insert((operation.into(), name.into()), 16);
}

pub(in super::super) fn operation(fake: &Fake, operation: &str, name: &str) -> io::Result<()> {
    let mut world = fake.borrow_mut();
    world.calls.push(format!("{operation}:{name}"));
    match world.failures.get(&(operation.into(), name.into())) {
        Some(errno) => Err(io::Error::from_raw_os_error(*errno)),
        None => Ok(()),
    }
}

pub(in super::super) fn released(fake: &Fake) {
    assert!(
        fake.borrow().opened.is_empty(),
        "all owned descriptors must close"
    );
    assert!(
        fake.borrow().grabbed.is_empty(),
        "all grabs must be released"
    );
}

#[derive(Debug)]
pub(in super::super) struct FakeHandle {
    pub world: Fake,
    pub name: String,
    pub generation: u64,
}

impl Drop for FakeHandle {
    fn drop(&mut self) {
        let mut world = self.world.borrow_mut();
        world.calls.push(format!("close:{}", self.name));
        world.opened.remove(&self.name);
        world.grabbed.remove(&self.name);
    }
}
