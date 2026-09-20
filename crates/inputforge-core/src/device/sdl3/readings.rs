//! Raw normalized samples; axis interpretation belongs to the engine.
use super::OpenDevice;
use crate::{
    device::InputUpdate,
    types::{AxisPolarity, AxisValue, HatDirection, InputAddress, InputEvent, InputId, InputValue},
};
use sdl3::joystick::{HatState, Joystick};
use std::time::Instant;

pub(super) fn axis_value(raw: i16) -> InputValue {
    InputValue::Axis {
        value: AxisValue::raw(normalize(raw)),
        polarity: AxisPolarity::default(),
    }
}
fn normalize(raw: i16) -> f64 {
    (f64::from(raw) / f64::from(i16::MAX)).clamp(-1.0, 1.0)
}

impl OpenDevice {
    pub(super) fn event(
        &self,
        native: &InputId,
        value: InputValue,
        timestamp: Instant,
    ) -> Option<InputEvent> {
        Some(InputEvent {
            source: InputAddress::Bound {
                device: self.device_id.clone(),
                input: super::bindings::index(&self.binding, native)?,
            },
            value,
            timestamp,
        })
    }
    pub(super) fn emit_sample(
        &mut self,
        native: u8,
        value: i16,
        out: &mut Vec<InputUpdate>,
    ) -> bool {
        let Some(InputId::Axis { index }) =
            super::bindings::index(&self.binding, &InputId::Axis { index: native })
        else {
            return false;
        };
        let requested = self.resample.remove(&index);
        if !self.sampled.insert(index) && !requested {
            return false;
        }
        out.push(InputUpdate::AxisSample {
            device: self.device_id.clone(),
            index,
            value: normalize(value),
        });
        true
    }
    pub(super) fn sample_value(&self, native: u8, current: i16) -> Option<i16> {
        let InputId::Axis { index } =
            super::bindings::index(&self.binding, &InputId::Axis { index: native })?
        else {
            return None;
        };
        if !self.binding.resolves(&InputId::Axis { index }) {
            return None;
        }
        let initialized = self.sampled.contains(&index);
        let requested = self.resample.contains(&index);
        if initialized && !requested {
            return None;
        }
        let initial = initial_value(&self.joystick, native);
        if requested {
            (current != 0 || initialized || initial.is_some()).then_some(current)
        } else {
            resting_sample(current, initial)
        }
    }
    pub(super) fn sample_pending(&mut self, now: Instant, out: &mut Vec<InputUpdate>) {
        let mut sampled = false;
        for index in 0..self.info.axes {
            if let Ok(raw) = self.joystick.axis(u32::from(index))
                && let Some(value) = self.sample_value(index, raw)
            {
                sampled |= self.emit_sample(index, value, out);
            }
        }
        if sampled {
            self.snapshot(now, out);
        }
    }
    pub(super) fn snapshot(&mut self, now: Instant, out: &mut Vec<InputUpdate>) {
        let mut values = Vec::new();
        for index in 0..self.info.axes {
            if let Ok(raw) = self.joystick.axis(u32::from(index)) {
                if let Some(value) = self.sample_value(index, raw) {
                    self.emit_sample(index, value, out);
                }
                values.extend(self.event(&InputId::Axis { index }, axis_value(raw), now));
            }
        }
        for index in 0..self.info.buttons {
            if let Ok(pressed) = self.joystick.button(u32::from(index)) {
                values.extend(self.event(
                    &InputId::Button { index },
                    InputValue::Button { pressed },
                    now,
                ));
            }
        }
        for index in 0..self.info.hats {
            if let Ok(state) = self.joystick.hat(u32::from(index)) {
                values.extend(self.event(
                    &InputId::Hat { index },
                    InputValue::Hat {
                        direction: sdl_hat_to_direction(state),
                    },
                    now,
                ));
            }
        }
        out.push(InputUpdate::Snapshot {
            device: self.device_id.clone(),
            values,
            recovered: false,
        });
    }
}
fn resting_sample(current: i16, initial: Option<i16>) -> Option<i16> {
    initial.or_else(|| (current != 0).then_some(current))
}
#[expect(
    unsafe_code,
    reason = "SDL exposes initial-state validity only through FFI"
)]
fn initial_value(joystick: &Joystick, index: u8) -> Option<i16> {
    // SAFETY: the joystick remains open throughout both calls, and index is bounded by num_axes.
    let raw = unsafe { sdl3::sys::joystick::SDL_GetJoystickFromID(joystick.id().into()) };
    let mut initial = 0;
    // SAFETY: raw is non-null, the handle is open, and initial is valid writable storage.
    let valid = !raw.is_null()
        && unsafe {
            sdl3::sys::joystick::SDL_GetJoystickAxisInitialState(
                raw,
                i32::from(index),
                &raw mut initial,
            )
        };
    valid.then_some(initial)
}

/// Convert SDL3 [`HatState`] to our [`HatDirection`].
pub(super) fn sdl_hat_to_direction(state: HatState) -> HatDirection {
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
    fn deferred_zero_is_not_a_resting_sample_and_values_stay_normalized() {
        assert_eq!(resting_sample(0, None), None);
        assert_eq!(resting_sample(0, Some(0)), Some(0));
        assert_eq!(resting_sample(0, Some(i16::MIN)), Some(i16::MIN));
        assert_eq!(resting_sample(i16::MAX, Some(i16::MIN)), Some(i16::MIN));
        assert_eq!(resting_sample(i16::MIN, None), Some(i16::MIN));
        assert!((normalize(i16::MIN) + 1.0).abs() < f64::EPSILON);
        assert!((normalize(i16::MAX) - 1.0).abs() < f64::EPSILON);
    }

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
