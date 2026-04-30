// Rust guideline compliant 2026-03-06

//! Per-device, per-axis calibration storage.

use std::collections::HashMap;

use crate::processing::calibration::Calibration;
use crate::types::DeviceId;

/// Stores calibration configurations indexed by device, then axis.
///
/// Each physical device axis can have its own [`Calibration`] to map raw
/// hardware values to normalized output. The two-level map avoids
/// cloning `DeviceId` on every lookup, critical since `get()` sits
/// on the ~1 kHz input hot path.
#[derive(Debug, Clone)]
pub struct DeviceCalibrationStore {
    calibrations: HashMap<DeviceId, HashMap<u8, Calibration>>,
}

impl DeviceCalibrationStore {
    /// Create an empty calibration store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            calibrations: HashMap::new(),
        }
    }

    /// Look up the calibration for a specific device axis.
    #[must_use]
    pub fn get(&self, device: &DeviceId, axis: u8) -> Option<&Calibration> {
        self.calibrations
            .get(device)
            .and_then(|axes| axes.get(&axis))
    }

    /// Insert or update the calibration for a specific device axis.
    pub fn set(&mut self, device: DeviceId, axis: u8, cal: Calibration) {
        self.calibrations
            .entry(device)
            .or_default()
            .insert(axis, cal);
    }

    /// Remove the calibration for a specific device axis.
    pub fn remove(&mut self, device: &DeviceId, axis: u8) -> Option<Calibration> {
        let axis_map = self.calibrations.get_mut(device)?;
        let removed = axis_map.remove(&axis);
        if axis_map.is_empty() {
            self.calibrations.remove(device);
        }
        removed
    }

    /// Return all calibrations for a device, sorted by axis index.
    #[must_use]
    pub fn get_for_device(&self, device: &DeviceId) -> Vec<(u8, &Calibration)> {
        let Some(axes) = self.calibrations.get(device) else {
            return Vec::new();
        };
        let mut entries: Vec<(u8, &Calibration)> =
            axes.iter().map(|(&axis, cal)| (axis, cal)).collect();
        entries.sort_by_key(|(axis, _)| *axis);
        entries
    }

    /// Return all device IDs that have calibrations.
    #[must_use]
    pub fn device_ids(&self) -> Vec<&DeviceId> {
        self.calibrations.keys().collect()
    }
}

impl Default for DeviceCalibrationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_calibration() -> Calibration {
        Calibration::new(-32768.0, -100.0, 100.0, 32767.0, true).unwrap()
    }

    fn device(name: &str) -> DeviceId {
        DeviceId(name.to_owned())
    }

    #[test]
    fn new_store_is_empty() {
        let store = DeviceCalibrationStore::new();
        assert!(store.get(&device("dev1"), 0).is_none());
        assert!(store.get_for_device(&device("dev1")).is_empty());
    }

    #[test]
    fn default_delegates_to_new() {
        let store = DeviceCalibrationStore::default();
        assert!(store.get_for_device(&device("any")).is_empty());
    }

    #[test]
    fn set_and_get() {
        let mut store = DeviceCalibrationStore::new();
        let cal = test_calibration();
        store.set(device("dev1"), 0, cal.clone());

        let retrieved = store.get(&device("dev1"), 0).unwrap();
        assert_eq!(retrieved, &cal);
    }

    #[test]
    fn get_returns_none_for_missing_axis() {
        let mut store = DeviceCalibrationStore::new();
        store.set(device("dev1"), 0, test_calibration());

        assert!(store.get(&device("dev1"), 1).is_none());
    }

    #[test]
    fn get_returns_none_for_missing_device() {
        let mut store = DeviceCalibrationStore::new();
        store.set(device("dev1"), 0, test_calibration());

        assert!(store.get(&device("dev2"), 0).is_none());
    }

    #[test]
    fn set_overwrites_existing() {
        let mut store = DeviceCalibrationStore::new();
        let cal1 = Calibration::new(-100.0, -10.0, 10.0, 100.0, true).unwrap();
        let cal2 = Calibration::new(-200.0, -20.0, 20.0, 200.0, false).unwrap();

        store.set(device("dev1"), 0, cal1);
        store.set(device("dev1"), 0, cal2.clone());

        let retrieved = store.get(&device("dev1"), 0).unwrap();
        assert_eq!(retrieved, &cal2);
    }

    #[test]
    fn remove_existing() {
        let mut store = DeviceCalibrationStore::new();
        let cal = test_calibration();
        store.set(device("dev1"), 0, cal.clone());

        let removed = store.remove(&device("dev1"), 0);
        assert_eq!(removed, Some(cal));
        assert!(store.get(&device("dev1"), 0).is_none());
    }

    #[test]
    fn remove_cleans_up_empty_device_entry() {
        let mut store = DeviceCalibrationStore::new();
        store.set(device("dev1"), 0, test_calibration());
        store.remove(&device("dev1"), 0);
        assert!(store.device_ids().is_empty());
    }

    #[test]
    fn remove_missing_returns_none() {
        let mut store = DeviceCalibrationStore::new();
        assert!(store.remove(&device("dev1"), 0).is_none());
    }

    #[test]
    fn get_for_device_returns_sorted_axes() {
        let mut store = DeviceCalibrationStore::new();
        let cal = test_calibration();

        // Insert in non-sorted order
        store.set(device("dev1"), 3, cal.clone());
        store.set(device("dev1"), 0, cal.clone());
        store.set(device("dev1"), 7, cal.clone());
        store.set(device("dev1"), 1, cal.clone());

        let entries = store.get_for_device(&device("dev1"));
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].0, 0);
        assert_eq!(entries[1].0, 1);
        assert_eq!(entries[2].0, 3);
        assert_eq!(entries[3].0, 7);
    }

    #[test]
    fn get_for_device_excludes_other_devices() {
        let mut store = DeviceCalibrationStore::new();
        let cal = test_calibration();

        store.set(device("dev1"), 0, cal.clone());
        store.set(device("dev2"), 0, cal.clone());
        store.set(device("dev1"), 1, cal.clone());

        let entries = store.get_for_device(&device("dev1"));
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn device_ids_returns_all_devices() {
        let mut store = DeviceCalibrationStore::new();
        let cal = test_calibration();

        store.set(device("dev1"), 0, cal.clone());
        store.set(device("dev2"), 1, cal.clone());

        let ids = store.device_ids();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn clone_produces_independent_copy() {
        let mut store = DeviceCalibrationStore::new();
        store.set(device("dev1"), 0, test_calibration());

        let cloned = store.clone();
        store.remove(&device("dev1"), 0);

        assert!(store.get(&device("dev1"), 0).is_none());
        assert!(cloned.get(&device("dev1"), 0).is_some());
    }

    #[test]
    fn debug_format() {
        let store = DeviceCalibrationStore::new();
        let debug = format!("{store:?}");
        assert!(debug.contains("DeviceCalibrationStore"));
    }
}
