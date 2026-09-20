// Rust guideline compliant 2026-03-03

use std::collections::HashSet;

use crate::error::Result;
use crate::profile::controllers::{AxisBinding, DeviceBinding};
use crate::types::{AxisPolarity, DeviceId, DeviceInfo, InputEvent};

use super::InputUpdate;
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

    fn binding_table(&self, id: &DeviceId) -> Result<Option<DeviceBinding>> {
        Ok(self
            .devices
            .iter()
            .find(|d| &d.id == id)
            .map(|info| DeviceBinding {
                observed_layout: None,
                device: info.id.clone(),
                axes: (0..info.axes)
                    .map(|index| AxisBinding {
                        code: u16::from(index),
                        minimum: i32::from(i16::MIN),
                        maximum: i32::from(i16::MAX),
                        polarity: AxisPolarity::default(),
                    })
                    .collect(),
                buttons: (0..info.buttons).map(u16::from).collect(),
                hats: (0..info.hats).collect(),
                unavailable: Vec::new(),
            }))
    }

    fn poll(&mut self, out: &mut Vec<InputUpdate>) -> Result<()> {
        if !self.events.is_empty() {
            out.push(InputUpdate::Frame(std::mem::take(&mut self.events)));
        }
        Ok(())
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

    fn list_hidden_devices(&self) -> Result<Vec<String>> {
        Ok(self
            .hidden_devices
            .iter()
            .filter_map(|d| d.instance_path.clone())
            .collect())
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
            instance_path: None,
            axis_polarities: vec![],
        }
    }

    fn sample_event() -> InputEvent {
        InputEvent {
            source: InputAddress::Bound {
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
        let mut polled = Vec::new();
        source.poll(&mut polled).unwrap();
        assert_eq!(polled.len(), 1);
        let mut empty = Vec::new();
        source.poll(&mut empty).unwrap();
        assert!(empty.is_empty(), "poll should drain events");
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
