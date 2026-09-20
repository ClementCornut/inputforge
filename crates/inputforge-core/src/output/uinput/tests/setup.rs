use super::{ABS_SETUP, DEV_SETUP, SET_PHYS};
use nix::{
    libc,
    sys::ioctl::{SIZEMASK, SIZESHIFT},
};

#[test]
fn uinput_requests_match_linux_uapi_structure_and_pointer_sizes() {
    // Linux uinput.h specifies char* for UI_SET_PHYS: using sizeof(char)
    // reproduces evdev 0.13.2's EINVAL before any device can be created.
    assert_eq!(
        (SET_PHYS >> SIZESHIFT) & SIZEMASK,
        size_of::<*const libc::c_char>() as libc::c_ulong
    );
    assert_eq!((DEV_SETUP >> SIZESHIFT) & SIZEMASK, 92);
    assert_eq!((ABS_SETUP >> SIZESHIFT) & SIZEMASK, 28);
    assert_eq!(std::mem::offset_of!(libc::uinput_abs_setup, absinfo), 4);
}
