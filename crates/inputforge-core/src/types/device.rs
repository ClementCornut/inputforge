// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use super::address::VJoyAxis;

/// Stable identifier for a physical device, persists across reconnects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

/// Metadata about a physical device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub axes: u8,
    pub buttons: u8,
    pub hats: u8,
    /// Platform-specific device path for hiding subsystems (e.g., `HidHide`).
    ///
    /// On Windows this is the HID device interface path returned by SDL3.
    /// `None` when the platform path is unavailable.
    pub instance_path: Option<String>,
}

/// Configuration for creating a virtual vJoy device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualDeviceConfig {
    pub device_id: u8,
    pub axes: Vec<VJoyAxis>,
    pub button_count: u8,
    pub hat_count: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_equality() {
        let a = DeviceId("abc123".to_owned());
        let b = DeviceId("abc123".to_owned());
        let c = DeviceId("xyz789".to_owned());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn device_info_serde_roundtrip() {
        let info = DeviceInfo {
            id: DeviceId("guid-001".to_owned()),
            name: "Throttle".to_owned(),
            axes: 3,
            buttons: 12,
            hats: 1,
            instance_path: Some("HID\\VID_045E&PID_02FF".to_owned()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: DeviceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, back);
    }

    #[test]
    fn device_info_serde_roundtrip_no_instance_path() {
        let info = DeviceInfo {
            id: DeviceId("guid-002".to_owned()),
            name: "Pedals".to_owned(),
            axes: 3,
            buttons: 0,
            hats: 0,
            instance_path: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: DeviceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, back);
    }

    #[test]
    fn virtual_device_config_serde_roundtrip() {
        let config = VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X, VJoyAxis::Y, VJoyAxis::Z],
            button_count: 32,
            hat_count: 1,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: VirtualDeviceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, back);
    }
}
