//! SDL event ordering and native-to-frozen input indices.
use super::{
    Sdl3Input,
    readings::{axis_value, sdl_hat_to_direction},
};
use crate::{
    device::{HotplugEvent, InputUpdate},
    types::{InputId, InputValue},
};
use sdl3::event::Event;
use std::time::Instant;

impl Sdl3Input {
    pub(super) fn dispatch(&mut self, event: &Event, now: Instant, out: &mut Vec<InputUpdate>) {
        let (which, input, value) = match *event {
            Event::JoyDeviceAdded { which, .. } => {
                self.try_open_joystick(which);
                return;
            }
            Event::JoyDeviceRemoved { which, .. } => {
                if let Some(device) = self.open_devices.remove(&which) {
                    out.push(InputUpdate::Reset {
                        device: device.device_id.clone(),
                    });
                    self.hotplug_buffer
                        .push(HotplugEvent::Disconnected(device.device_id));
                }
                return;
            }
            Event::JoyAxisMotion {
                which,
                axis_idx,
                value,
                ..
            } => {
                if let Some(device) = self.open_devices.get_mut(&which)
                    && !device.pending_snapshot
                    && let Some(sample) = device.sample_value(axis_idx, value)
                {
                    device.emit_sample(axis_idx, sample, out);
                }
                (which, InputId::Axis { index: axis_idx }, axis_value(value))
            }
            Event::JoyButtonDown {
                which, button_idx, ..
            } => (
                which,
                InputId::Button { index: button_idx },
                InputValue::Button { pressed: true },
            ),
            Event::JoyButtonUp {
                which, button_idx, ..
            } => (
                which,
                InputId::Button { index: button_idx },
                InputValue::Button { pressed: false },
            ),
            Event::JoyHatMotion {
                which,
                hat_idx,
                state,
                ..
            } => (
                which,
                InputId::Hat { index: hat_idx },
                InputValue::Hat {
                    direction: sdl_hat_to_direction(state),
                },
            ),
            _ => return,
        };
        if let Some(device) = self.open_devices.get(&which) {
            // Initial SDL button/hat events describe held state, not fresh user actions.
            if !device.pending_snapshot
                && let Some(event) = device.event(&input, value, now)
            {
                out.push(InputUpdate::Frame(vec![event]));
            }
        }
    }
}
