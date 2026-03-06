// Rust guideline compliant 2026-03-03

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

use super::address::VJoyAxis;

/// Whether an axis is bipolar (centered at 0) or unipolar (rests at min).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AxisPolarity {
    /// Axis rests at center (0). Range displayed as −100 %..+100 %.
    #[default]
    Bipolar,
    /// Axis rests at minimum (−1.0). Range displayed as 0 %..100 %.
    Unipolar,
}

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
    /// Per-axis polarity detected at enumeration time.
    ///
    /// Length equals `axes`. Empty or shorter than `axes` means all remaining
    /// axes default to [`AxisPolarity::Bipolar`].
    #[serde(default)]
    pub axis_polarities: Vec<AxisPolarity>,
}

/// Configuration for creating a virtual vJoy device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualDeviceConfig {
    pub device_id: u8,
    pub axes: Vec<VJoyAxis>,
    pub button_count: u8,
    pub hat_count: u8,
}

impl VirtualDeviceConfig {
    /// Validates that the device configuration is within valid bounds.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] if `device_id` is 0 or greater than 16.
    pub fn validate(&self) -> Result<()> {
        if self.device_id == 0 || self.device_id > 16 {
            return Err(EngineError::InvalidConfig {
                reason: format!("vJoy device_id must be 1..=16, got {}", self.device_id),
            });
        }
        Ok(())
    }
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
            axis_polarities: vec![],
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
            axis_polarities: vec![],
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

    #[test]
    fn validate_accepts_valid_device_ids() {
        for id in 1..=16 {
            let config = VirtualDeviceConfig {
                device_id: id,
                axes: vec![],
                button_count: 0,
                hat_count: 0,
            };
            assert!(config.validate().is_ok(), "device_id {id} should be valid");
        }
    }

    #[test]
    fn validate_rejects_device_id_zero() {
        let config = VirtualDeviceConfig {
            device_id: 0,
            axes: vec![],
            button_count: 0,
            hat_count: 0,
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("1..=16"),
            "error should mention valid range: {err}"
        );
    }

    #[test]
    fn validate_rejects_device_id_above_16() {
        let config = VirtualDeviceConfig {
            device_id: 17,
            axes: vec![],
            button_count: 0,
            hat_count: 0,
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("1..=16"),
            "error should mention valid range: {err}"
        );
    }
}
