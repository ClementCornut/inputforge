//! Lazy vJoy output sessions; input monitoring does not require the driver.
use super::traits::OutputSink;
use crate::{
    error::{EngineError, Result},
    types::{HatDirection, VJoyAxis, VirtualDeviceConfig},
};
use std::collections::{HashMap, HashSet};
use std::fmt;
use vjoy::{ButtonState, Device, VJoy};
mod native;
use native::{axis_value_to_vjoy, hat_direction_to_vjoy, vjoy_axis_id};

/// Cached native output state, owned only between `start` and `stop`.
#[derive(Default)]
pub struct VJoyOutput {
    vjoy: Option<VJoy>,
    pub(super) active_devices: HashSet<u8>,
    cached_states: HashMap<u8, Device>,
    dirty_devices: HashSet<u8>,
}
impl fmt::Debug for VJoyOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VJoyOutput")
            .field("active_devices", &self.active_devices)
            .finish_non_exhaustive()
    }
}
impl VJoyOutput {
    /// Create an idle sink without loading or acquiring the vJoy driver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    fn active(&mut self, device: u8) -> Result<&mut Device> {
        self.cached_states
            .get_mut(&device)
            .ok_or(EngineError::VJoyDeviceUnavailable { device_id: device })
    }
    fn clear(&mut self) {
        self.cached_states.clear();
        self.active_devices.clear();
        self.dirty_devices.clear();
        // VJoy::drop relinquishes every device acquired by its constructor.
        self.vjoy = None;
    }
}
fn load_driver() -> Result<VJoy> {
    VJoy::from_default_dll_location().map_err(|error| {
        tracing::debug!(?error, "vJoy driver load failed");
        EngineError::VJoyDriverMissing
    })
}
impl OutputSink for VJoyOutput {
    fn start(&mut self, configs: &[VirtualDeviceConfig]) -> Result<()> {
        if self.vjoy.is_some() {
            return Err(EngineError::OutputFailed {
                reason: "Stop before changing virtual devices".into(),
            });
        }
        let vjoy = load_driver()?;
        let available = native::configs(&vjoy);
        let mut states = HashMap::new();
        for config in configs {
            if !available.iter().any(|native| native.supports(config))
                || states.contains_key(&config.device_id)
            {
                return Err(EngineError::VJoyDeviceUnavailable {
                    device_id: config.device_id,
                });
            }
            let state = vjoy
                .get_device_state(u32::from(config.device_id))
                .map_err(|_| EngineError::VJoyDeviceUnavailable {
                    device_id: config.device_id,
                })?;
            // vjoy 0.7 indexes its device vector by ID-1; reject sparse-ID mismatches.
            if state.id() != u32::from(config.device_id) {
                return Err(EngineError::VJoyDeviceUnavailable {
                    device_id: config.device_id,
                });
            }
            states.insert(config.device_id, state);
        }
        self.active_devices = states.keys().copied().collect();
        self.cached_states = states;
        self.vjoy = Some(vjoy);
        if let Err(error) = self.neutralize_session() {
            let cleanup = self.neutralize_session();
            self.clear();
            return Err(EngineError::OutputFailed {
                reason: match cleanup {
                    Ok(()) => error.to_string(),
                    Err(cleanup) => format!("{error}; cleanup: {cleanup}"),
                },
            });
        }
        Ok(())
    }
    fn neutralize(&mut self) -> Result<()> {
        self.neutralize_session()
    }
    fn stop(&mut self) -> Result<()> {
        let result = self.neutralize_session();
        self.clear();
        result
    }
    fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()> {
        let axis_id = vjoy_axis_id(axis);
        self.active(device)?
            .set_axis(axis_id, axis_value_to_vjoy(value))
            .map_err(|error| EngineError::OutputFailed {
                reason: format!("vJoy device {device} axis {axis:?}: {error:?}"),
            })?;
        self.dirty_devices.insert(device);
        Ok(())
    }
    fn set_button(&mut self, device: u8, button: u8, pressed: bool) -> Result<()> {
        self.active(device)?
            .set_button(
                button,
                if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
            )
            .map_err(|error| EngineError::OutputFailed {
                reason: format!("vJoy device {device} button {button}: {error:?}"),
            })?;
        self.dirty_devices.insert(device);
        Ok(())
    }
    fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) -> Result<()> {
        self.active(device)?
            .set_hat(hat, hat_direction_to_vjoy(direction))
            .map_err(|error| EngineError::OutputFailed {
                reason: format!("vJoy device {device} hat {hat}: {error:?}"),
            })?;
        self.dirty_devices.insert(device);
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        let Some(vjoy) = self.vjoy.as_mut() else {
            return Ok(());
        };
        let mut first_error = None;
        self.dirty_devices.retain(|device| {
            let state = self
                .cached_states
                .get(device)
                .expect("dirty vJoy device has an active cached state");
            match vjoy.update_device_state(state) {
                Ok(()) => false,
                Err(error) => {
                    first_error.get_or_insert_with(|| EngineError::OutputFailed {
                        reason: format!("vJoy device {device}: {error:?}"),
                    });
                    true
                }
            }
        });
        first_error.map_or(Ok(()), Err)
    }
    fn list_devices(&self) -> Vec<VirtualDeviceConfig> {
        if let Some(vjoy) = &self.vjoy {
            return native::configs(vjoy);
        }
        // Discovery acquires temporarily because the dependency exposes no read-only inventory.
        load_driver().map_or_else(|_| Vec::new(), |vjoy| native::configs(&vjoy))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inactive_setters_never_load_or_acquire_the_driver() {
        let mut output = VJoyOutput::new();
        assert!(output.set_axis(1, VJoyAxis::X, 0.0).is_err());
        assert!(output.set_button(1, 1, true).is_err());
        assert!(output.set_hat(1, 1, HatDirection::N).is_err());
        assert!(output.vjoy.is_none());
        assert!(output.stop().is_ok());
    }
}
