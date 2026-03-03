// Rust guideline compliant 2026-03-02

use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

use sdl3::event::Event;
use sdl3::joystick::{HatState, Joystick, JoystickId};
use sdl3::{EventPump, JoystickSubsystem, Sdl};

use crate::error::{EngineError, Result};
use crate::types::{
    AxisValue, DeviceId, DeviceInfo, HatDirection, InputAddress, InputEvent, InputId, InputValue,
};

use super::traits::{HotplugEvent, InputSource};

/// SDL3-based physical device input reader.
///
/// Wraps the SDL3 joystick subsystem and translates SDL events into
/// [`InputEvent`] values.  Hotplug detection is handled automatically
/// via SDL3's `JoyDeviceAdded` / `JoyDeviceRemoved` events.
///
/// # Thread Safety
///
/// This type is `!Send` because the underlying SDL3 context must be
/// used from the thread that created it. Do not attempt to move this
/// value across thread boundaries.
pub struct Sdl3Input {
    // Keep the SDL context alive for the lifetime of this struct.
    _sdl: Sdl,
    joystick_subsystem: JoystickSubsystem,
    event_pump: EventPump,
    /// Maps the `u32` returned by [`Joystick::id`] to an opened handle
    /// and our stable [`DeviceId`].
    open_devices: HashMap<u32, OpenDevice>,
    /// Buffered hotplug events to be drained by the caller.
    hotplug_buffer: Vec<HotplugEvent>,
}

impl fmt::Debug for Sdl3Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sdl3Input")
            .field("open_devices", &self.open_devices.len())
            .field("hotplug_buffer", &self.hotplug_buffer.len())
            .finish_non_exhaustive()
    }
}

struct OpenDevice {
    joystick: Joystick,
    device_id: DeviceId,
}

impl Sdl3Input {
    /// Initialize SDL3 with the joystick subsystem.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Sdl`] if SDL3 fails to initialize.
    pub fn new() -> Result<Self> {
        let sdl = sdl3::init().map_err(|e| EngineError::Sdl(e.to_string()))?;
        let joystick_subsystem = sdl
            .joystick()
            .map_err(|e| EngineError::Sdl(e.to_string()))?;
        let event_pump = sdl
            .event_pump()
            .map_err(|e| EngineError::Sdl(e.to_string()))?;

        let mut input = Self {
            _sdl: sdl,
            joystick_subsystem,
            event_pump,
            open_devices: HashMap::new(),
            hotplug_buffer: Vec::new(),
        };

        // Open all already-connected joysticks.
        input.open_all_connected();
        Ok(input)
    }

    /// Open all currently connected joysticks that aren't already open.
    fn open_all_connected(&mut self) {
        let ids: Vec<JoystickId> = match self.joystick_subsystem.joysticks() {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!("failed to enumerate joysticks: {e}");
                return;
            }
        };

        for id in ids {
            self.try_open_joystick(id);
        }
    }

    /// Attempt to open a joystick by its SDL joystick ID.
    fn try_open_joystick(&mut self, sdl_id: JoystickId) {
        match self.joystick_subsystem.open(sdl_id) {
            Ok(joystick) => {
                let instance_id = joystick.id();
                if self.open_devices.contains_key(&instance_id) {
                    return;
                }
                let guid = joystick.guid();
                let device_id = DeviceId(guid.string());
                let info = device_info_from_joystick(&joystick, &device_id);
                self.hotplug_buffer.push(HotplugEvent::Connected(info));
                self.open_devices.insert(
                    instance_id,
                    OpenDevice {
                        joystick,
                        device_id,
                    },
                );
            }
            Err(e) => {
                tracing::warn!("failed to open joystick: {e}");
            }
        }
    }

    /// Handle a joystick removal event.
    fn handle_device_removed(&mut self, instance_id: u32) {
        if let Some(removed) = self.open_devices.remove(&instance_id) {
            self.hotplug_buffer
                .push(HotplugEvent::Disconnected(removed.device_id));
        }
    }
}

impl InputSource for Sdl3Input {
    fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        self.open_devices
            .values()
            .map(|d| device_info_from_joystick(&d.joystick, &d.device_id))
            .collect()
    }

    fn poll(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();
        let now = Instant::now();

        // Collect all SDL events first to release the mutable borrow on
        // `event_pump` before we need `&mut self` for hotplug handling.
        let sdl_events: Vec<Event> = self.event_pump.poll_iter().collect();

        for event in sdl_events {
            match event {
                Event::JoyAxisMotion {
                    which,
                    axis_idx,
                    value,
                    ..
                } => {
                    if let Some(device) = self.open_devices.get(&which) {
                        events.push(InputEvent {
                            source: InputAddress {
                                device: device.device_id.clone(),
                                input: InputId::Axis { index: axis_idx },
                            },
                            value: InputValue::Axis {
                                value: AxisValue::raw(f64::from(value)),
                            },
                            timestamp: now,
                        });
                    }
                }
                Event::JoyButtonDown {
                    which, button_idx, ..
                } => {
                    if let Some(device) = self.open_devices.get(&which) {
                        events.push(InputEvent {
                            source: InputAddress {
                                device: device.device_id.clone(),
                                input: InputId::Button { index: button_idx },
                            },
                            value: InputValue::Button { pressed: true },
                            timestamp: now,
                        });
                    }
                }
                Event::JoyButtonUp {
                    which, button_idx, ..
                } => {
                    if let Some(device) = self.open_devices.get(&which) {
                        events.push(InputEvent {
                            source: InputAddress {
                                device: device.device_id.clone(),
                                input: InputId::Button { index: button_idx },
                            },
                            value: InputValue::Button { pressed: false },
                            timestamp: now,
                        });
                    }
                }
                Event::JoyHatMotion {
                    which,
                    hat_idx,
                    state,
                    ..
                } => {
                    if let Some(device) = self.open_devices.get(&which) {
                        events.push(InputEvent {
                            source: InputAddress {
                                device: device.device_id.clone(),
                                input: InputId::Hat { index: hat_idx },
                            },
                            value: InputValue::Hat {
                                direction: sdl_hat_to_direction(state),
                            },
                            timestamp: now,
                        });
                    }
                }
                Event::JoyDeviceAdded { which, .. } => {
                    // `which` in JoyDeviceAdded is the joystick ID to pass
                    // to `open()` (wrapped as `SDL_JoystickID`).
                    self.try_open_joystick(sdl3::sys::joystick::SDL_JoystickID(which));
                }
                Event::JoyDeviceRemoved { which, .. } => {
                    self.handle_device_removed(which);
                }
                _ => {}
            }
        }

        events
    }

    fn is_device_connected(&self, id: &DeviceId) -> bool {
        self.open_devices
            .values()
            .any(|d| d.device_id == *id && d.joystick.connected())
    }

    fn hotplug_events(&mut self) -> Vec<HotplugEvent> {
        std::mem::take(&mut self.hotplug_buffer)
    }
}

/// Build a [`DeviceInfo`] from an open SDL3 joystick.
///
/// Calls `SDL_GetJoystickPathForID` via FFI to retrieve the platform-specific
/// device path (used by `HidHide` on Windows).
#[expect(unsafe_code, reason = "SDL_GetJoystickPathForID is an SDL3 FFI call")]
fn device_info_from_joystick(joystick: &Joystick, device_id: &DeviceId) -> DeviceInfo {
    let instance_id = sdl3::sys::joystick::SDL_JoystickID(joystick.id());

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

    DeviceInfo {
        id: device_id.clone(),
        name: joystick.name(),
        axes: u8::try_from(joystick.num_axes()).unwrap_or(u8::MAX),
        buttons: u8::try_from(joystick.num_buttons()).unwrap_or(u8::MAX),
        hats: u8::try_from(joystick.num_hats()).unwrap_or(u8::MAX),
        instance_path,
    }
}

/// Convert SDL3 [`HatState`] to our [`HatDirection`].
pub(crate) fn sdl_hat_to_direction(state: HatState) -> HatDirection {
    match state {
        HatState::Centered => HatDirection::Center,
        HatState::Up => HatDirection::N,
        HatState::RightUp => HatDirection::NE,
        HatState::Right => HatDirection::E,
        HatState::RightDown => HatDirection::SE,
        HatState::Down => HatDirection::S,
        HatState::LeftDown => HatDirection::SW,
        HatState::Left => HatDirection::W,
        HatState::LeftUp => HatDirection::NW,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdl_hat_to_direction_covers_all_variants() {
        let cases = [
            (HatState::Centered, HatDirection::Center),
            (HatState::Up, HatDirection::N),
            (HatState::RightUp, HatDirection::NE),
            (HatState::Right, HatDirection::E),
            (HatState::RightDown, HatDirection::SE),
            (HatState::Down, HatDirection::S),
            (HatState::LeftDown, HatDirection::SW),
            (HatState::Left, HatDirection::W),
            (HatState::LeftUp, HatDirection::NW),
        ];

        for (sdl_state, expected) in cases {
            assert_eq!(sdl_hat_to_direction(sdl_state), expected);
        }
    }
}
