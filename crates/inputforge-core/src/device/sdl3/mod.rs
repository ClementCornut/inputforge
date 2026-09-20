//! SDL input monitoring with ordered snapshots, events and disconnect resets.
use super::{HotplugEvent, InputSource, InputUpdate};
use crate::{
    error::{EngineError, Result},
    profile::controllers::DeviceBinding,
    types::{DeviceId, DeviceInfo, InputId},
};
use sdl3::{
    EventPump, JoystickSubsystem, Sdl,
    joystick::{Joystick, JoystickId},
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    time::Instant,
};
mod bindings;
mod events;
mod metadata;
mod readings;
use metadata::{device_info_from_joystick, diagnostics_from_joystick};

/// Physical joystick monitoring. Must remain on the SDL initialization thread.
pub struct Sdl3Input {
    // Handles must close before the SDL context is dropped.
    open_devices: HashMap<JoystickId, OpenDevice>,
    event_pump: EventPump,
    joystick_subsystem: JoystickSubsystem,
    _sdl: Sdl,
    hotplug_buffer: Vec<HotplugEvent>,
    bindings: Vec<DeviceBinding>,
    poll_count: u32,
}
struct OpenDevice {
    joystick: Joystick,
    device_id: DeviceId,
    info: DeviceInfo,
    binding: DeviceBinding,
    sampled: HashSet<u8>,
    resample: HashSet<u8>,
    pending_snapshot: bool,
}
impl fmt::Debug for Sdl3Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sdl3Input")
            .field("open_devices", &self.open_devices.len())
            .finish_non_exhaustive()
    }
}
impl Sdl3Input {
    /// Initialize SDL joystick monitoring without starting an output session.
    /// # Errors
    /// Returns an SDL initialization error.
    pub fn new() -> Result<Self> {
        let sdl = sdl3::init().map_err(|e| EngineError::Sdl(e.to_string()))?;
        let joystick_subsystem = sdl
            .joystick()
            .map_err(|e| EngineError::Sdl(e.to_string()))?;
        let event_pump = sdl
            .event_pump()
            .map_err(|e| EngineError::Sdl(e.to_string()))?;
        let mut input = Self {
            open_devices: HashMap::new(),
            event_pump,
            joystick_subsystem,
            _sdl: sdl,
            hotplug_buffer: Vec::new(),
            bindings: Vec::new(),
            poll_count: 0,
        };
        input.refresh()?;
        Ok(input)
    }
    fn try_open_joystick(&mut self, id: JoystickId) {
        if self.open_devices.contains_key(&id) {
            return;
        }
        let joystick = match self.joystick_subsystem.open(id) {
            Ok(joystick) => joystick,
            Err(error) => {
                tracing::warn!(%error, "failed to open joystick");
                return;
            }
        };
        let device_id = DeviceId(joystick.guid().string());
        let info = device_info_from_joystick(&joystick, &device_id);
        let diagnostics = diagnostics_from_joystick(&joystick);
        if own_output(&info.name, diagnostics.vendor_id, diagnostics.product_id) {
            return;
        }
        let discovered = bindings::binding(&info);
        let mut binding = self
            .bindings
            .iter()
            .find(|b| b.device == device_id)
            .cloned()
            .unwrap_or_else(|| discovered.clone());
        binding.reconcile(&discovered, false);
        self.hotplug_buffer.push(HotplugEvent::Connected {
            info: info.clone(),
            diagnostics,
        });
        self.open_devices.insert(
            id,
            OpenDevice {
                joystick,
                device_id,
                info,
                binding,
                sampled: HashSet::new(),
                resample: HashSet::new(),
                pending_snapshot: true,
            },
        );
    }
}
impl InputSource for Sdl3Input {
    fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        self.open_devices.values().map(|d| d.info.clone()).collect()
    }
    fn configure(&mut self, bindings: &[DeviceBinding]) -> Result<()> {
        self.bindings = bindings.to_vec();
        for device in self.open_devices.values_mut() {
            let discovered = bindings::binding(&device.info);
            if let Some(saved) = bindings.iter().find(|b| b.device == device.device_id) {
                device.binding = saved.clone();
            }
            device.binding.reconcile(&discovered, false);
        }
        Ok(())
    }
    fn binding_table(&self, id: &DeviceId) -> Result<Option<DeviceBinding>> {
        Ok(self
            .open_devices
            .values()
            .find(|d| &d.device_id == id)
            .map(|d| d.binding.clone()))
    }
    fn confirm_binding(&mut self, id: &DeviceId, input: &InputId) -> Result<DeviceBinding> {
        let device = self
            .open_devices
            .values_mut()
            .find(|d| &d.device_id == id)
            .ok_or_else(|| EngineError::InvalidConfig {
                reason: "Controller is not connected".into(),
            })?;
        let table =
            bindings::confirm(&mut device.binding, &bindings::binding(&device.info), input)?;
        if let Some(saved) = self.bindings.iter_mut().find(|b| &b.device == id) {
            *saved = table.clone();
        } else {
            self.bindings.push(table.clone());
        }
        Ok(table)
    }
    fn refresh(&mut self) -> Result<()> {
        for id in self
            .joystick_subsystem
            .joysticks()
            .map_err(|e| EngineError::Sdl(e.to_string()))?
        {
            self.try_open_joystick(id);
        }
        Ok(())
    }
    fn poll(&mut self, out: &mut Vec<InputUpdate>) -> Result<()> {
        let now = Instant::now();
        let events: Vec<_> = self.event_pump.poll_iter().collect();
        for event in events {
            self.dispatch(&event, now, out);
        }
        self.poll_count = self.poll_count.wrapping_add(1);
        for device in self.open_devices.values_mut() {
            if device.pending_snapshot {
                device.snapshot(now, out);
                device.pending_snapshot = false;
            } else if self.poll_count.is_multiple_of(100) {
                // At a 1 ms poll interval, retry deferred native initialization every 100 ms.
                device.sample_pending(now, out);
            }
        }
        Ok(())
    }
    fn request_axis_sample(&mut self, id: &DeviceId, index: u8) -> Result<()> {
        if let Some(device) = self.open_devices.values_mut().find(|d| &d.device_id == id) {
            device.resample.insert(index);
            // Explicit redetection uses the next current reading, never the old initial state.
        }
        Ok(())
    }
    fn is_device_connected(&self, id: &DeviceId) -> bool {
        self.open_devices
            .values()
            .any(|d| &d.device_id == id && d.joystick.connected())
    }
    fn hotplug_events(&mut self) -> Vec<HotplugEvent> {
        std::mem::take(&mut self.hotplug_buffer)
    }
}
fn own_output(name: &str, vendor: Option<u16>, product: Option<u16>) -> bool {
    // vJoy's published VID/PID; InputForge uinput outputs carry our product name.
    (vendor == Some(0x1234) && product == Some(0xbead))
        || name.to_ascii_lowercase().contains("vjoy")
        || name.starts_with("InputForge Virtual")
}
#[cfg(test)]
mod tests {
    use super::own_output;
    #[test]
    fn virtual_outputs_never_feed_back_as_physical_input() {
        assert!(own_output("vJoy Device", None, None));
        assert!(own_output("renamed", Some(0x1234), Some(0xbead)));
        assert!(own_output("InputForge Virtual Controller 1", None, None));
        assert!(!own_output("Physical pedals", Some(1), Some(2)));
    }
}
