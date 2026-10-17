//! Test-only raw seed adaptation through the real app bridge projection.
use crate::snapshots::{Frame, LiveSnapshot, MappingKey};
use inputforge_core::{
    state::AppState,
    types::{AxisValue, InputAddress, InputId, InputValue},
};
pub(crate) fn prepare(
    state: &mut AppState,
    key: &MappingKey,
    seed: Option<&LiveSnapshot>,
) -> Frame {
    if let Some(seed) = seed {
        for (device, values) in state.devices.iter().zip(&seed.device_inputs) {
            for (index, (value, polarity)) in values.axes.iter().enumerate() {
                state.input_cache.update(
                    &InputAddress::Bound {
                        device: device.info.id.clone(),
                        input: InputId::Axis {
                            index: u8::try_from(index).unwrap(),
                        },
                    },
                    &InputValue::Axis {
                        value: AxisValue::new(*value),
                        polarity: *polarity,
                    },
                );
            }
            for (index, pressed) in values.buttons.iter().enumerate() {
                state.input_cache.update(
                    &InputAddress::Bound {
                        device: device.info.id.clone(),
                        input: InputId::Button {
                            index: u8::try_from(index).unwrap(),
                        },
                    },
                    &InputValue::Button { pressed: *pressed },
                );
            }
            for (index, direction) in values.hats.iter().enumerate() {
                state.input_cache.update(
                    &InputAddress::Bound {
                        device: device.info.id.clone(),
                        input: InputId::Hat {
                            index: u8::try_from(index).unwrap(),
                        },
                    },
                    &InputValue::Hat {
                        direction: *direction,
                    },
                );
            }
        }
        for (device, values) in state.virtual_devices.iter().zip(&seed.output_values) {
            for (axis, value) in &values.axes {
                state.output_cache.set_axis(device.device_id, *axis, *value);
            }
            for (index, pressed) in values.buttons.iter().enumerate() {
                state.output_cache.set_button(
                    device.device_id,
                    u8::try_from(index + 1).unwrap(),
                    *pressed,
                );
            }
            for (index, direction) in values.hats.iter().enumerate() {
                state.output_cache.set_hat(
                    device.device_id,
                    u8::try_from(index + 1).unwrap(),
                    *direction,
                );
            }
        }
        for activity in &seed.output_activity {
            state.output_activity.record(
                activity.owner.clone(),
                activity.value,
                std::time::Instant::now(),
                false,
            );
        }
    }
    inputforge_app::bridge::prepare(state, Some(key))
}
