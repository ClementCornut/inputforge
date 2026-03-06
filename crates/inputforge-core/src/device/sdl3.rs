// Rust guideline compliant 2026-03-03

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::marker::PhantomData;
use std::time::Instant;

use sdl3::event::Event;
use sdl3::joystick::{HatState, Joystick, JoystickId};
use sdl3::{EventPump, JoystickSubsystem, Sdl};

use crate::error::{EngineError, Result};
use crate::types::{
    AxisPolarity, AxisValue, DeviceId, DeviceInfo, HatDirection, InputAddress, InputEvent, InputId,
    InputValue,
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
    /// Axes whose polarity has been classified from a real hardware
    /// event.  Key is `(instance_id, axis_index)`.  Once an axis is in
    /// this set its polarity is final and won't be re-evaluated.
    classified_axes: HashSet<(u32, u8)>,
    /// Number of poll cycles elapsed.  Used to trigger a deferred
    /// polarity re-probe after SDL3 has had time to populate hardware
    /// axis values via DirectInput.
    poll_count: u32,
    /// SDL3 context is not thread-safe; this marker makes the `!Send`
    /// bound explicit instead of relying implicitly on SDL types.
    _not_send: PhantomData<*mut ()>,
}

impl fmt::Debug for Sdl3Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sdl3Input")
            .field("open_devices", &self.open_devices.len())
            .field("hotplug_buffer", &self.hotplug_buffer.len())
            .field("classified_axes", &self.classified_axes.len())
            .finish_non_exhaustive()
    }
}

struct OpenDevice {
    joystick: Joystick,
    device_id: DeviceId,
    info: DeviceInfo,
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
            classified_axes: HashSet::new(),
            poll_count: 0,
            _not_send: PhantomData,
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
                tracing::error!("failed to enumerate joysticks: {e}");
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
                self.hotplug_buffer
                    .push(HotplugEvent::Connected(info.clone()));
                self.open_devices.insert(
                    instance_id,
                    OpenDevice {
                        joystick,
                        device_id,
                        info,
                    },
                );
            }
            Err(e) => {
                tracing::warn!("failed to open joystick: {e}");
            }
        }
    }

    /// Re-probe axis polarities using `SDL_GetJoystickAxis` after the
    /// event pump has had time to populate real hardware values.
    ///
    /// Any axes reclassified as unipolar trigger a
    /// `HotplugEvent::Connected` update so the engine and GUI pick up
    /// the corrected polarity.
    #[expect(unsafe_code, reason = "SDL3 FFI calls for axis re-probe")]
    #[expect(
        clippy::cast_sign_loss,
        reason = "axis_idx iterates from 0..num_axes, always non-negative"
    )]
    fn deferred_reprobe_polarities(&mut self) {
        for device in self.open_devices.values_mut() {
            let instance_id = sdl3::sys::joystick::SDL_JoystickID(device.joystick.id());
            // SAFETY: the joystick is open so the pointer is valid.
            let raw = unsafe { sdl3::sys::joystick::SDL_GetJoystickFromID(instance_id) };
            if raw.is_null() {
                continue;
            }
            let mut changed = false;
            for axis_idx in 0..i32::from(device.info.axes) {
                let idx = axis_idx as usize;
                // SAFETY: raw is non-null and valid.
                let current = unsafe { sdl3::sys::joystick::SDL_GetJoystickAxis(raw, axis_idx) };
                let polarity = if current < UNIPOLAR_INITIAL_STATE_THRESHOLD {
                    AxisPolarity::Unipolar
                } else {
                    AxisPolarity::Bipolar
                };
                if idx < device.info.axis_polarities.len()
                    && device.info.axis_polarities[idx] != polarity
                {
                    tracing::info!(
                        device = %device.info.name,
                        axis_idx,
                        current,
                        ?polarity,
                        "deferred reprobe reclassified axis polarity"
                    );
                    device.info.axis_polarities[idx] = polarity;
                    changed = true;
                }
            }
            if changed {
                self.hotplug_buffer
                    .push(HotplugEvent::Connected(device.info.clone()));
            }
        }
    }

    /// Handle a joystick removal event.
    fn handle_device_removed(&mut self, instance_id: u32) {
        if let Some(removed) = self.open_devices.remove(&instance_id) {
            self.classified_axes.retain(|&(id, _)| id != instance_id);
            self.hotplug_buffer
                .push(HotplugEvent::Disconnected(removed.device_id));
        }
    }
}

impl InputSource for Sdl3Input {
    fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        self.open_devices.values().map(|d| d.info.clone()).collect()
    }

    fn poll(&mut self, out: &mut Vec<InputEvent>) {
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
                    if let Some(device) = self.open_devices.get_mut(&which) {
                        // Lazy polarity classification: on the first event
                        // for each axis, check the raw value to determine
                        // if it's a unipolar axis (pedal/trigger resting
                        // at −32 768).
                        let key = (which, axis_idx);
                        if self.classified_axes.insert(key) {
                            let polarity = if value < UNIPOLAR_INITIAL_STATE_THRESHOLD {
                                AxisPolarity::Unipolar
                            } else {
                                AxisPolarity::Bipolar
                            };
                            let idx = usize::from(axis_idx);
                            if idx < device.info.axis_polarities.len()
                                && device.info.axis_polarities[idx] != polarity
                            {
                                tracing::info!(
                                    device = %device.info.name,
                                    axis_idx,
                                    value,
                                    ?polarity,
                                    "reclassified axis polarity from first event"
                                );
                                device.info.axis_polarities[idx] = polarity;
                                self.hotplug_buffer
                                    .push(HotplugEvent::Connected(device.info.clone()));
                            }
                        }
                        out.push(InputEvent {
                            source: InputAddress {
                                device: device.device_id.clone(),
                                input: InputId::Axis { index: axis_idx },
                            },
                            value: InputValue::Axis {
                                value: AxisValue::raw(f64::from(value) / f64::from(i16::MAX)),
                            },
                            timestamp: now,
                        });
                    }
                }
                Event::JoyButtonDown {
                    which, button_idx, ..
                } => {
                    if let Some(device) = self.open_devices.get(&which) {
                        out.push(InputEvent {
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
                        out.push(InputEvent {
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
                        out.push(InputEvent {
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

        // Deferred polarity re-probe: after a few poll cycles the event
        // pump has run enough for DirectInput to populate real axis
        // resting values.  We re-read current values via SDL FFI and
        // reclassify any axes that were misdetected at enumeration time.
        self.poll_count = self.poll_count.saturating_add(1);
        if self.poll_count >= REPROBE_START
            && self.poll_count <= REPROBE_END
            && self.poll_count % REPROBE_INTERVAL == 0
        {
            self.deferred_reprobe_polarities();
        }
    }

    fn is_device_connected(&self, id: &DeviceId) -> bool {
        self.open_devices
            .values()
            .any(|d| d.device_id == *id && d.joystick.connected())
    }

    fn hotplug_events(&mut self) -> Vec<HotplugEvent> {
        self.hotplug_buffer.drain(..).collect()
    }
}

/// Axis initial-state threshold below which an axis is classified as unipolar.
///
/// SDL3 reports axis values as `i16` in `[-32 768, 32 767]`.  Pedals and
/// triggers typically rest at the minimum value (`-32 768`).  Any axis whose
/// initial state is below this threshold (bottom 25 % of the signed range) is
/// treated as unipolar so the GUI can display it as 0–100 % instead of
/// −100 %..+100 %.
const UNIPOLAR_INITIAL_STATE_THRESHOLD: i16 = -16_384;

/// First poll cycle at which deferred polarity re-probing starts.
const REPROBE_START: u32 = 5;

/// Last poll cycle at which re-probing is attempted (2 s at 1 ms/tick).
const REPROBE_END: u32 = 2000;

/// Interval between re-probe attempts within the window.
///
/// With `POLL_INTERVAL = 1 ms` this means one probe every 100 ms,
/// giving ~20 attempts total to catch DirectInput populating the
/// real axis resting values.
const REPROBE_INTERVAL: u32 = 100;

/// Build a [`DeviceInfo`] from an open SDL3 joystick.
///
/// Calls SDL3 FFI to retrieve the platform-specific device path (used by
/// `HidHide` on Windows) and to probe per-axis initial state for polarity
/// detection.
#[expect(unsafe_code, reason = "SDL3 FFI calls for path and axis initial state")]
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

    let num_axes = u8::try_from(joystick.num_axes()).unwrap_or(u8::MAX);

    // SAFETY: `SDL_GetJoystickFromID` returns the `SDL_Joystick` pointer
    // associated with the given instance ID, or null if invalid.  We hold
    // an open `Joystick` reference so the pointer is valid for the
    // duration of this function.
    let raw_joystick = unsafe { sdl3::sys::joystick::SDL_GetJoystickFromID(instance_id) };

    // Best-effort polarity detection at enumeration time.  On Windows
    // with DirectInput, SDL3 often reports 0 for all axes at this point
    // because hardware hasn't been polled yet.  The runtime lazy
    // classifier in `poll()` will correct any misclassifications when
    // the first real axis event arrives.
    let axis_polarities = detect_axis_polarities(raw_joystick, num_axes);

    DeviceInfo {
        id: device_id.clone(),
        name: joystick.name(),
        axes: num_axes,
        buttons: u8::try_from(joystick.num_buttons()).unwrap_or(u8::MAX),
        hats: u8::try_from(joystick.num_hats()).unwrap_or(u8::MAX),
        instance_path,
        axis_polarities,
    }
}

/// Best-effort polarity detection via `SDL_GetJoystickAxis`.
///
/// On Windows with DirectInput, this usually returns all-zero at
/// enumeration time.  The deferred re-probe in
/// [`Sdl3Input::deferred_reprobe_polarities`] and the lazy classifier
/// in [`Sdl3Input::poll`] handle the real classification once hardware
/// values are available.
#[expect(unsafe_code, reason = "SDL_GetJoystickAxis is an SDL3 FFI call")]
fn detect_axis_polarities(
    raw_joystick: *mut sdl3::sys::joystick::SDL_Joystick,
    num_axes: u8,
) -> Vec<AxisPolarity> {
    (0..i32::from(num_axes))
        .map(|axis_idx| {
            if raw_joystick.is_null() {
                return AxisPolarity::Bipolar;
            }
            // SAFETY: `raw_joystick` is non-null and valid (checked above).
            // `SDL_GetJoystickAxis` returns 0 on failure.
            let current_value =
                unsafe { sdl3::sys::joystick::SDL_GetJoystickAxis(raw_joystick, axis_idx) };
            if current_value < UNIPOLAR_INITIAL_STATE_THRESHOLD {
                AxisPolarity::Unipolar
            } else {
                AxisPolarity::Bipolar
            }
        })
        .collect()
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
