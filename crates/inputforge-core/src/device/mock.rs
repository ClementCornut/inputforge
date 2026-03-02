// Rust guideline compliant 2026-03-02

use std::collections::HashSet;

use crate::error::Result;
use crate::types::{DeviceId, DeviceInfo, InputEvent};

use super::traits::{DeviceHider, HotplugEvent, InputSource};

/// Mock implementation of [`InputSource`] for testing.
///
/// Pre-load `devices`, `events`, and `hotplug` fields before calling
/// trait methods. [`InputSource::poll`] drains `events`;
/// [`InputSource::hotplug_events`] drains `hotplug`.
#[derive(Debug, Default)]
pub struct MockInputSource {
    pub devices: Vec<DeviceInfo>,
    pub events: Vec<InputEvent>,
    pub hotplug: Vec<HotplugEvent>,
    pub connected: HashSet<DeviceId>,
}

impl InputSource for MockInputSource {
    fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        self.devices.clone()
    }

    fn poll(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.events)
    }

    fn is_device_connected(&self, id: &DeviceId) -> bool {
        self.connected.contains(id)
    }

    fn hotplug_events(&mut self) -> Vec<HotplugEvent> {
        std::mem::take(&mut self.hotplug)
    }
}

/// Mock implementation of [`DeviceHider`] for testing.
///
/// Records hide/unhide calls and tracks an `active` flag.
#[derive(Debug, Default)]
pub struct MockDeviceHider {
    pub hidden_devices: Vec<DeviceInfo>,
    pub active: bool,
}

impl DeviceHider for MockDeviceHider {
    fn hide_device(&mut self, device: &DeviceInfo) -> Result<()> {
        self.hidden_devices.push(device.clone());
        Ok(())
    }

    fn unhide_device(&mut self, device: &DeviceInfo) -> Result<()> {
        self.hidden_devices.retain(|d| d.id != device.id);
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use crate::types::{InputAddress, InputId, InputValue};

    use super::*;

    fn sample_device() -> DeviceInfo {
        DeviceInfo {
            id: DeviceId("guid-001".to_owned()),
            name: "Test Joystick".to_owned(),
            axes: 4,
            buttons: 12,
            hats: 1,
        }
    }

    fn sample_event() -> InputEvent {
        InputEvent {
            source: InputAddress {
                device: DeviceId("guid-001".to_owned()),
                input: InputId::Button { index: 0 },
            },
            value: InputValue::Button { pressed: true },
            timestamp: Instant::now(),
        }
    }

    #[test]
    fn mock_input_source_enumerate_returns_stored_devices() {
        let source = MockInputSource {
            devices: vec![sample_device()],
            ..Default::default()
        };
        assert_eq!(source.enumerate_devices().len(), 1);
        assert_eq!(source.enumerate_devices()[0].name, "Test Joystick");
    }

    #[test]
    fn mock_input_source_poll_drains_events() {
        let mut source = MockInputSource {
            events: vec![sample_event()],
            ..Default::default()
        };
        let polled = source.poll();
        assert_eq!(polled.len(), 1);
        assert!(source.poll().is_empty(), "poll should drain events");
    }

    #[test]
    fn mock_input_source_is_device_connected() {
        let mut source = MockInputSource::default();
        let id = DeviceId("guid-001".to_owned());
        assert!(!source.is_device_connected(&id));
        source.connected.insert(id.clone());
        assert!(source.is_device_connected(&id));
    }

    #[test]
    fn mock_input_source_hotplug_drains() {
        let mut source = MockInputSource {
            hotplug: vec![HotplugEvent::Disconnected(DeviceId("guid-001".to_owned()))],
            ..Default::default()
        };
        let events = source.hotplug_events();
        assert_eq!(events.len(), 1);
        assert!(
            source.hotplug_events().is_empty(),
            "hotplug should drain events"
        );
    }

    #[test]
    fn mock_device_hider_tracks_hidden_devices() {
        let mut hider = MockDeviceHider::default();
        let device = sample_device();

        hider.hide_device(&device).unwrap();
        assert_eq!(hider.hidden_devices.len(), 1);

        hider.unhide_device(&device).unwrap();
        assert!(hider.hidden_devices.is_empty());
    }

    #[test]
    fn mock_device_hider_is_active() {
        let mut hider = MockDeviceHider::default();
        assert!(!hider.is_active());
        hider.active = true;
        assert!(hider.is_active());
    }
}
