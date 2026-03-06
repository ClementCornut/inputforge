// Rust guideline compliant 2026-03-03

use crate::types::DeviceInfo;

/// Live device state tracked by the engine.
///
/// Combines static device metadata with runtime connection status.
/// Updated by the engine when hotplug events occur.
#[derive(Debug, Clone)]
pub struct DeviceState {
    /// Static device metadata (name, axis/button/hat counts).
    pub info: DeviceInfo,
    /// Whether the device is currently connected.
    pub connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DeviceId;

    fn sample_device() -> DeviceInfo {
        DeviceInfo {
            id: DeviceId("test-device".to_owned()),
            name: "Test Joystick".to_owned(),
            axes: 4,
            buttons: 12,
            hats: 1,
            instance_path: None,
            axis_polarities: vec![],
        }
    }

    #[test]
    fn device_state_construction() {
        let state = DeviceState {
            info: sample_device(),
            connected: true,
        };
        assert!(state.connected);
        assert_eq!(state.info.name, "Test Joystick");
    }

    #[test]
    fn device_state_clone() {
        let state = DeviceState {
            info: sample_device(),
            connected: false,
        };
        let cloned = state.clone();
        assert_eq!(cloned.connected, false);
        assert_eq!(cloned.info.id, state.info.id);
    }

    #[test]
    fn device_state_debug_format() {
        let state = DeviceState {
            info: sample_device(),
            connected: true,
        };
        let debug = format!("{state:?}");
        assert!(debug.contains("DeviceState"));
        assert!(debug.contains("connected: true"));
    }
}
