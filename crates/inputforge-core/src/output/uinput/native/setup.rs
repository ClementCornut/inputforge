use super::super::{Error, config};
use super::Handle;
use crate::types::VirtualDeviceConfig;
use nix::libc;
use std::{
    ffi::CString,
    fs::OpenOptions,
    io,
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::fs::OpenOptionsExt,
    },
};

// UI_SET_PHYS is _IOW('U', 108, char*), not char. evdev 0.13.2's
// with_phys encodes sizeof(char), which the kernel rejects with EINVAL.
const SET_PHYS: libc::c_ulong =
    nix::request_code_write!(b'U', 108, size_of::<*const libc::c_char>());
const DEV_SETUP: libc::c_ulong = nix::request_code_write!(b'U', 3, size_of::<libc::uinput_setup>());
const ABS_SETUP: libc::c_ulong =
    nix::request_code_write!(b'U', 4, size_of::<libc::uinput_abs_setup>());

#[expect(
    unsafe_code,
    reason = "Linux uinput setup ioctls use owned descriptors and initialized native ABI structures"
)]
pub(super) fn create(cfg: &VirtualDeviceConfig) -> Result<Handle, Error> {
    let slot = cfg.device_id;
    let error = |operation, source| Error::io(operation, Some(slot), "/dev/uinput", source);
    // Rust OpenOptions provides close-on-exec. The descriptor stays nonblocking
    // for its entire lifetime; any setup failure closes it and discards setup.
    let fd: OwnedFd = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/uinput")
        .map_err(|e| error("open uinput", e))?
        .into();
    let phys = CString::new(config::phys(slot)).expect("generated physical path has no NUL");
    // SAFETY: SET_PHYS copies a terminated C string synchronously from a live allocation.
    result(unsafe { libc::ioctl(fd.as_raw_fd(), SET_PHYS, phys.as_ptr()) })
        .map_err(|e| error("set physical path", e))?;
    set_bit(&fd, 100, 1).map_err(|e| error("set key event capability", e))?;
    for button in 1..=cfg.button_count {
        set_bit(
            &fd,
            101,
            config::button_code(button).expect("validated button"),
        )
        .map_err(|e| error("set button capability", e))?;
    }
    let axes = config::axes(cfg);
    if !axes.is_empty() {
        set_bit(&fd, 100, 3).map_err(|e| error("set absolute event capability", e))?;
    }
    for (code, minimum, maximum) in axes {
        let setup = libc::uinput_abs_setup {
            code,
            absinfo: libc::input_absinfo {
                value: 0,
                minimum,
                maximum,
                fuzz: 0,
                flat: 0,
                resolution: 0,
            },
        };
        // SAFETY: UI_ABS_SETUP reads the live ABI struct; its padding is ignored by
        // the kernel. It also enables the axis bit, so UI_SET_ABSBIT is unnecessary.
        result(unsafe { libc::ioctl(fd.as_raw_fd(), ABS_SETUP, &raw const setup) })
            .map_err(|e| error("set axis capability", e))?;
    }
    let mut setup = libc::uinput_setup {
        id: libc::input_id {
            bustype: 6,
            vendor: 0,
            product: u16::from(slot),
            version: 1,
        },
        name: [0; 80],
        ff_effects_max: 0,
    };
    for (target, byte) in setup.name.iter_mut().zip(config::name(slot).bytes()) {
        *target = libc::c_char::try_from(byte).expect("generated name is ASCII");
    }
    // SAFETY: DEV_SETUP reads this live native struct with a terminated name.
    result(unsafe { libc::ioctl(fd.as_raw_fd(), DEV_SETUP, &raw const setup) })
        .map_err(|e| error("set device identity", e))?;
    // SAFETY: UI_DEV_CREATE takes no argument. No fallible operation follows before
    // transferring the descriptor to the destruction guard.
    result(unsafe { libc::ioctl(fd.as_raw_fd(), nix::request_code_none!(b'U', 1)) })
        .map_err(|e| error("create uinput device", e))?;
    Ok(Handle::Native(Some(fd)))
}

#[expect(
    unsafe_code,
    reason = "uinput capability ioctls take integer values, not pointers"
)]
fn set_bit(fd: &OwnedFd, number: u8, value: u16) -> io::Result<()> {
    let request = nix::request_code_write!(b'U', number, size_of::<libc::c_int>());
    // SAFETY: callers select UI_SET_EVBIT or UI_SET_KEYBIT; both accept an integer.
    result(unsafe { libc::ioctl(fd.as_raw_fd(), request, libc::c_int::from(value)) })
}

fn result(result: libc::c_int) -> io::Result<()> {
    nix::errno::Errno::result(result)
        .map(|_| ())
        .map_err(io::Error::from)
}

#[cfg(test)]
#[path = "../tests/setup.rs"]
mod tests;
