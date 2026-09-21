use super::{
    super::device::{DeviceKey, DeviceSpec},
    sysfs,
};
use evdev::{AbsInfo, InputId, raw_stream::RawDevice};
use std::{
    ffi::CStr,
    fs::OpenOptions,
    io,
    os::fd::{AsRawFd, OwnedFd},
    os::unix::{
        ffi::OsStrExt,
        fs::{FileTypeExt, OpenOptionsExt},
    },
    path::PathBuf,
};

pub(super) fn check(device: &OwnedFd, spec: &DeviceSpec) -> io::Result<()> {
    let syspath = syspath(device)?;
    for entry in std::fs::read_dir(&syspath)? {
        let entry = entry?;
        if !entry.file_name().as_bytes().starts_with(b"event") {
            continue;
        }
        let record = udev::Device::from_syspath(&entry.path())?;
        if !record.is_initialized() {
            continue;
        }
        let node = record
            .devnode()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "udev event node pending"))?;
        if !std::fs::metadata(node)?.file_type().is_char_device() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "event node is not a character device",
            ));
        }
        let metadata = if matches!(spec.key, DeviceKey::Controller(_)) {
            event_metadata(node, &record, spec)?
        } else {
            sysfs::read(&syspath, &record, spec)?
        };
        verify(&metadata, spec)?;
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "initialized udev event node pending",
    ))
}

fn event_metadata(
    node: &std::path::Path,
    record: &udev::Device,
    spec: &DeviceSpec,
) -> io::Result<Metadata> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NONBLOCK)
        .open(node)?;
    let raw = RawDevice::from_fd(file.into())?;
    Ok(Metadata {
        name: raw.name().map(str::to_owned),
        phys: raw.physical_path().map(str::to_owned),
        id: raw.input_id(),
        keys: raw
            .supported_keys()
            .into_iter()
            .flat_map(|set| set.iter())
            .map(|key| key.0)
            .collect(),
        relatives: raw
            .supported_relative_axes()
            .into_iter()
            .flat_map(|set| set.iter())
            .map(|axis| axis.0)
            .collect(),
        axes: raw
            .get_absinfo()?
            .map(|(code, info)| (code.0, info))
            .collect(),
        events: raw.supported_events().iter().map(|event| event.0).collect(),
        class: record.property_value(spec.required_class).map(|value| {
            (
                spec.required_class.to_owned(),
                value.to_string_lossy().into_owned(),
            )
        }),
    })
}

pub(in crate::output::uinput) struct Metadata {
    pub(in crate::output::uinput) name: Option<String>,
    pub(in crate::output::uinput) phys: Option<String>,
    pub(in crate::output::uinput) id: InputId,
    pub(in crate::output::uinput) keys: Vec<u16>,
    pub(in crate::output::uinput) relatives: Vec<u16>,
    pub(in crate::output::uinput) axes: Vec<(u16, AbsInfo)>,
    pub(in crate::output::uinput) events: Vec<u16>,
    pub(in crate::output::uinput) class: Option<(String, String)>,
}

pub(in crate::output::uinput) fn verify(device: &Metadata, spec: &DeviceSpec) -> io::Result<()> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "created output identity/capabilities differ from configuration",
        )
    };
    let id = &device.id;
    if device.name.as_deref() != Some(spec.name.as_str())
        || device.phys.as_deref() != Some(spec.phys.as_str())
        || id != &spec.id
    {
        return Err(invalid());
    }
    if device.keys != spec.keys || device.relatives != spec.relatives {
        return Err(invalid());
    }
    let mut expected = spec.absolutes.iter();
    for (code, info) in &device.axes {
        let Some(axis) = expected.next() else {
            return Err(invalid());
        };
        if (axis.code, axis.minimum, axis.maximum) != (*code, info.minimum(), info.maximum())
            || info.fuzz() != 0
            || info.flat() != 0
            || info.resolution() != 0
        {
            return Err(invalid());
        }
    }
    if expected.next().is_some() {
        return Err(invalid());
    }
    if device.events != spec.events()
        || !device
            .class
            .as_ref()
            .is_some_and(|(key, value)| key == spec.required_class && value == "1")
    {
        return Err(invalid());
    }
    Ok(())
}

#[expect(
    unsafe_code,
    reason = "UI_GET_SYSNAME writes at most the encoded buffer length"
)]
fn syspath(device: &OwnedFd) -> io::Result<PathBuf> {
    let mut buffer = [0_u8; 80];
    let request = nix::request_code_read!(b'U', 44, buffer.len());
    // SAFETY: the descriptor is live, and the mutable buffer has the encoded
    // capacity. The kernel writes a terminated inputN name within that capacity.
    let result = unsafe { nix::libc::ioctl(device.as_raw_fd(), request, buffer.as_mut_ptr()) };
    nix::errno::Errno::result(result)?;
    let name = CStr::from_bytes_until_nul(&buffer)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(PathBuf::from("/sys/devices/virtual/input")
        .join(std::ffi::OsStr::from_bytes(name.to_bytes())))
}

#[cfg(test)]
#[path = "../tests/readiness.rs"]
mod tests;
