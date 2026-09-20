use super::super::config;
use crate::types::VirtualDeviceConfig;
use evdev::{AbsInfo, BusType, InputId, raw_stream::RawDevice};
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

pub(super) fn check(device: &OwnedFd, cfg: &VirtualDeviceConfig) -> io::Result<()> {
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
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_NONBLOCK)
            .open(node)?;
        if !file.metadata()?.file_type().is_char_device() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "event node is not a character device",
            ));
        }
        let raw = RawDevice::from_fd(file.into())?;
        verify(
            &Metadata {
                name: raw.name().map(str::to_owned),
                phys: raw.physical_path().map(str::to_owned),
                id: raw.input_id(),
                keys: raw
                    .supported_keys()
                    .into_iter()
                    .flat_map(|set| set.iter())
                    .map(|key| key.0)
                    .collect(),
                axes: raw
                    .get_absinfo()?
                    .map(|(code, info)| (code.0, info))
                    .collect(),
                events: raw.supported_events().iter().map(|event| event.0).collect(),
            },
            cfg,
        )?;
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "initialized udev event node pending",
    ))
}

struct Metadata {
    name: Option<String>,
    phys: Option<String>,
    id: InputId,
    keys: Vec<u16>,
    axes: Vec<(u16, AbsInfo)>,
    events: Vec<u16>,
}

fn verify(device: &Metadata, cfg: &VirtualDeviceConfig) -> io::Result<()> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "created event node identity/capabilities differ from configuration",
        )
    };
    let id = &device.id;
    if device.name.as_deref() != Some(config::name(cfg.device_id).as_str())
        || device.phys.as_deref() != Some(config::phys(cfg.device_id).as_str())
        || id.bus_type() != BusType::BUS_VIRTUAL
        || id.vendor() != 0
        || id.product() != u16::from(cfg.device_id)
        || id.version() != 1
    {
        return Err(invalid());
    }
    let actual = device.keys.iter().copied();
    let expected =
        (1..=cfg.button_count).map(|id| config::button_code(id).expect("validated button"));
    if !actual.eq(expected) {
        return Err(invalid());
    }
    let mut axes = config::axes(cfg);
    axes.sort_by_key(|axis| axis.0);
    let mut expected = axes.into_iter();
    for (code, info) in &device.axes {
        if expected.next() != Some((*code, info.minimum(), info.maximum()))
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
    let expected = if cfg.axes.is_empty() && cfg.hat_count == 0 {
        vec![0, 1]
    } else {
        vec![0, 1, 3]
    };
    if device.events != expected {
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
