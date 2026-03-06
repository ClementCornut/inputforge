// Rust guideline compliant 2026-03-06

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use crate::processing::Calibration;
use crate::types::DeviceId;

/// A stable profile identifier backed by UUID v4.
///
/// Auto-generated on creation, preserved across renames and saves.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileId(String);

impl ProfileId {
    /// Generate a new random profile ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Return the string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProfileId {
    fn default() -> Self {
        Self::new()
    }
}

/// A device entry in the profile, associating a device GUID with a
/// human-readable name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceEntry {
    pub id: DeviceId,
    pub name: String,
}

/// Profile-level settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSettings {
    pub(super) startup_mode: String,
}

impl ProfileSettings {
    /// Return the startup mode name.
    #[must_use]
    pub fn startup_mode(&self) -> &str {
        &self.startup_mode
    }
}

/// Serializable calibration entry for a specific device axis.
///
/// Fields are public following the DTO pattern — this type exists
/// solely for serialization/deserialization between profile TOML
/// and the validated [`Calibration`] domain type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationEntry {
    pub device: DeviceId,
    pub axis: u8,
    pub physical_min: f64,
    pub physical_center_low: f64,
    pub physical_center_high: f64,
    pub physical_max: f64,
    pub enabled: bool,
}

impl CalibrationEntry {
    /// Convert this entry into a validated [`Calibration`].
    ///
    /// # Errors
    ///
    /// Returns an error if the calibration values violate the invariant
    /// `physical_min < physical_center_low <= physical_center_high < physical_max`.
    pub fn to_calibration(&self) -> Result<Calibration> {
        Calibration::new(
            self.physical_min,
            self.physical_center_low,
            self.physical_center_high,
            self.physical_max,
            self.enabled,
        )
    }

    /// Create a `CalibrationEntry` from a validated [`Calibration`].
    #[must_use]
    pub fn from_calibration(device: DeviceId, axis: u8, cal: &Calibration) -> Self {
        Self {
            device,
            axis,
            physical_min: cal.physical_min(),
            physical_center_low: cal.physical_center_low(),
            physical_center_high: cal.physical_center_high(),
            physical_max: cal.physical_max(),
            enabled: cal.enabled(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_id_generates_unique() {
        let id1 = ProfileId::new();
        let id2 = ProfileId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn profile_id_default() {
        let id = ProfileId::default();
        assert!(!id.as_str().is_empty());
    }

    #[test]
    fn device_entry_serde_roundtrip() {
        let entry = DeviceEntry {
            id: DeviceId("guid-001".to_owned()),
            name: "Left Stick".to_owned(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: DeviceEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, back);
    }

    #[test]
    fn profile_settings_getter() {
        let settings = ProfileSettings {
            startup_mode: "Default".to_owned(),
        };
        assert_eq!(settings.startup_mode(), "Default");
    }

    #[test]
    fn calibration_entry_roundtrip() {
        let entry = CalibrationEntry {
            device: DeviceId("dev-1".to_owned()),
            axis: 0,
            physical_min: -32768.0,
            physical_center_low: -100.0,
            physical_center_high: 100.0,
            physical_max: 32767.0,
            enabled: true,
        };
        let cal = entry.to_calibration().unwrap();
        let back = CalibrationEntry::from_calibration(DeviceId("dev-1".to_owned()), 0, &cal);
        assert_eq!(entry, back);
    }

    #[test]
    fn calibration_entry_invalid_values() {
        let entry = CalibrationEntry {
            device: DeviceId("dev-1".to_owned()),
            axis: 0,
            physical_min: 100.0,
            physical_center_low: 0.0,
            physical_center_high: 0.0,
            physical_max: -100.0,
            enabled: true,
        };
        assert!(entry.to_calibration().is_err());
    }

    #[test]
    fn calibration_entry_serde_roundtrip() {
        let entry = CalibrationEntry {
            device: DeviceId("dev-1".to_owned()),
            axis: 2,
            physical_min: -500.0,
            physical_center_low: -10.0,
            physical_center_high: 10.0,
            physical_max: 500.0,
            enabled: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: CalibrationEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, back);
    }
}
