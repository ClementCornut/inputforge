//! Native SDL inventory and diagnostics.
use crate::types::{DeviceConnectionState, DeviceDiagnostics, DeviceId, DeviceInfo};
use sdl3::joystick::Joystick;

/// Build a [`DeviceInfo`] from an open SDL3 joystick.
///
/// Calls SDL3 FFI to retrieve the platform-specific device path (used by
/// `HidHide` on Windows).
#[expect(unsafe_code, reason = "SDL3 FFI calls for native device paths")]
pub(super) fn device_info_from_joystick(joystick: &Joystick, device_id: &DeviceId) -> DeviceInfo {
    let instance_id = joystick.id().into();

    // SAFETY: `instance_id` was obtained from an open joystick via `id()`.
    // `SDL_GetJoystickPathForID` returns a pointer to an SDL-managed
    // null-terminated C string, or null if no path is available. The
    // returned string remains valid until the joystick subsystem is shut down.
    let path_ptr = unsafe { sdl3::sys::joystick::SDL_GetJoystickPathForID(instance_id) };

    let instance_path = if path_ptr.is_null() {
        None
    } else {
        // SAFETY: pointer is non-null and SDL guarantees it is a valid
        // null-terminated C string.
        Some(
            unsafe { std::ffi::CStr::from_ptr(path_ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    };

    let num_axes = u8::try_from(joystick.num_axes()).unwrap_or(u8::MAX);

    DeviceInfo {
        id: device_id.clone(),
        name: joystick.name(),
        axes: num_axes,
        buttons: u8::try_from(joystick.num_buttons()).unwrap_or(u8::MAX),
        hats: u8::try_from(joystick.num_hats()).unwrap_or(u8::MAX),
        instance_path,
        axis_polarities: Vec::new(),
    }
}

#[expect(
    unsafe_code,
    reason = "SDL3 FFI exposes diagnostics not wrapped by the joystick API"
)]
pub(super) fn diagnostics_from_joystick(joystick: &Joystick) -> DeviceDiagnostics {
    let instance_id = joystick.id().into();
    // SAFETY: `instance_id` was obtained from an open joystick via `id()`.
    let raw_joystick = unsafe { sdl3::sys::joystick::SDL_GetJoystickFromID(instance_id) };

    if raw_joystick.is_null() {
        return DeviceDiagnostics::default();
    }

    // SAFETY: `raw_joystick` is non-null and belongs to the open joystick handle.
    let vendor_id =
        nonzero_u16(unsafe { sdl3::sys::joystick::SDL_GetJoystickVendor(raw_joystick) });
    // SAFETY: `raw_joystick` is non-null and belongs to the open joystick handle.
    let product_id =
        nonzero_u16(unsafe { sdl3::sys::joystick::SDL_GetJoystickProduct(raw_joystick) });
    // SAFETY: `raw_joystick` is non-null and belongs to the open joystick handle.
    let product_version =
        nonzero_u16(unsafe { sdl3::sys::joystick::SDL_GetJoystickProductVersion(raw_joystick) });
    // SAFETY: `raw_joystick` is non-null and belongs to the open joystick handle.
    let firmware_version =
        nonzero_u16(unsafe { sdl3::sys::joystick::SDL_GetJoystickFirmwareVersion(raw_joystick) });
    // SAFETY: `raw_joystick` is non-null and belongs to the open joystick handle.
    let serial =
        serial_from_ptr(unsafe { sdl3::sys::joystick::SDL_GetJoystickSerial(raw_joystick) });
    // SAFETY: `raw_joystick` is non-null and belongs to the open joystick handle.
    let joystick_type = Some(joystick_type_label(unsafe {
        sdl3::sys::joystick::SDL_GetJoystickType(raw_joystick)
    }));
    // SAFETY: `raw_joystick` is non-null and belongs to the open joystick handle.
    let connection_state = Some(connection_state_from_sdl(unsafe {
        sdl3::sys::joystick::SDL_GetJoystickConnectionState(raw_joystick)
    }));

    DeviceDiagnostics {
        vendor_id,
        product_id,
        product_version,
        firmware_version,
        serial,
        joystick_type,
        connection_state,
        battery_percent: None,
        battery_state: None,
        is_virtual: None,
    }
}

fn nonzero_u16(value: u16) -> Option<u16> {
    if value == 0 { None } else { Some(value) }
}

#[expect(unsafe_code, reason = "SDL3 returns serial as a C string pointer")]
fn serial_from_ptr(ptr: *const std::ffi::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: SDL returns a null-terminated C string pointer or null.
    Some(
        unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn joystick_type_label(value: sdl3::sys::joystick::SDL_JoystickType) -> String {
    match value {
        sdl3::sys::joystick::SDL_JoystickType::UNKNOWN => "unknown",
        sdl3::sys::joystick::SDL_JoystickType::GAMEPAD => "gamepad",
        sdl3::sys::joystick::SDL_JoystickType::WHEEL => "wheel",
        sdl3::sys::joystick::SDL_JoystickType::ARCADE_STICK => "arcade_stick",
        sdl3::sys::joystick::SDL_JoystickType::FLIGHT_STICK => "flight_stick",
        sdl3::sys::joystick::SDL_JoystickType::DANCE_PAD => "dance_pad",
        sdl3::sys::joystick::SDL_JoystickType::GUITAR => "guitar",
        sdl3::sys::joystick::SDL_JoystickType::DRUM_KIT => "drum_kit",
        sdl3::sys::joystick::SDL_JoystickType::ARCADE_PAD => "arcade_pad",
        sdl3::sys::joystick::SDL_JoystickType::THROTTLE => "throttle",
        _ => "other",
    }
    .to_owned()
}

fn connection_state_from_sdl(
    value: sdl3::sys::joystick::SDL_JoystickConnectionState,
) -> DeviceConnectionState {
    match value {
        sdl3::sys::joystick::SDL_JoystickConnectionState::WIRED => DeviceConnectionState::Wired,
        sdl3::sys::joystick::SDL_JoystickConnectionState::WIRELESS => {
            DeviceConnectionState::Wireless
        }
        sdl3::sys::joystick::SDL_JoystickConnectionState::INVALID => DeviceConnectionState::Invalid,
        _ => DeviceConnectionState::Unknown,
    }
}
