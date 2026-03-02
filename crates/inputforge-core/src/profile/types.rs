// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}
