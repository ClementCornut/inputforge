use super::super::device::DeviceSpec;
use super::readiness::Metadata;
use crate::device::linux_bitmap;
use evdev::{BusType, InputId};
use std::{fs, io, path::Path};

pub(in crate::output::uinput) struct Attributes<'a> {
    pub(in crate::output::uinput) name: &'a str,
    pub(in crate::output::uinput) phys: &'a str,
    pub(in crate::output::uinput) bustype: &'a str,
    pub(in crate::output::uinput) vendor: &'a str,
    pub(in crate::output::uinput) product: &'a str,
    pub(in crate::output::uinput) version: &'a str,
    pub(in crate::output::uinput) events: &'a str,
    pub(in crate::output::uinput) keys: &'a str,
    pub(in crate::output::uinput) relatives: &'a str,
    pub(in crate::output::uinput) absolutes: &'a str,
    pub(in crate::output::uinput) class: Option<(&'a str, &'a str)>,
}

pub(super) fn read(parent: &Path, event: &udev::Device, spec: &DeviceSpec) -> io::Result<Metadata> {
    let name = text(parent, "name")?;
    let phys = text(parent, "phys")?;
    let bustype = text(parent, "id/bustype")?;
    let vendor = text(parent, "id/vendor")?;
    let product = text(parent, "id/product")?;
    let version = text(parent, "id/version")?;
    let events = text(parent, "capabilities/ev")?;
    let keys = text(parent, "capabilities/key")?;
    let relatives = text(parent, "capabilities/rel")?;
    let absolutes = text(parent, "capabilities/abs")?;
    let class = event
        .property_value(spec.required_class)
        .map(|value| {
            value
                .to_str()
                .map(|value| (spec.required_class, value))
                .ok_or_else(invalid)
        })
        .transpose()?;
    parse(&Attributes {
        name: &name,
        phys: &phys,
        bustype: &bustype,
        vendor: &vendor,
        product: &product,
        version: &version,
        events: &events,
        keys: &keys,
        relatives: &relatives,
        absolutes: &absolutes,
        class,
    })
}

pub(in crate::output::uinput) fn parse(attributes: &Attributes<'_>) -> io::Result<Metadata> {
    let axes = linux_bitmap::parse(attributes.absolutes, usize::BITS)?;
    if !axes.is_empty() {
        return Err(invalid());
    }
    Ok(Metadata {
        name: Some(attributes.name.to_owned()),
        phys: Some(attributes.phys.to_owned()),
        id: InputId::new(
            BusType(hex(attributes.bustype)?),
            hex(attributes.vendor)?,
            hex(attributes.product)?,
            hex(attributes.version)?,
        ),
        keys: bitmap(attributes.keys)?,
        relatives: bitmap(attributes.relatives)?,
        axes: Vec::new(),
        events: bitmap(attributes.events)?,
        class: attributes
            .class
            .map(|(key, value)| (key.to_owned(), value.to_owned())),
    })
}

fn text(parent: &Path, name: &str) -> io::Result<String> {
    fs::read_to_string(parent.join(name)).map(|mut value| {
        if value.ends_with('\n') {
            value.pop();
        }
        value
    })
}

fn hex(value: &str) -> io::Result<u16> {
    u16::from_str_radix(value, 16)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn bitmap(value: &str) -> io::Result<Vec<u16>> {
    linux_bitmap::parse(value, usize::BITS).map(|codes| codes.into_iter().collect())
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid sysfs input metadata")
}
