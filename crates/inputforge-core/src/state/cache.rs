// Rust guideline compliant 2026-03-03

use std::collections::HashMap;

use crate::pipeline::InputCache;
use crate::types::{DeviceId, HatDirection, InputAddress, InputValue};

/// Stores the latest value for every physical input.
///
/// Implements [`InputCache`] so it can be passed directly into
/// pipeline execution. The engine updates this cache on every
/// input event; the GUI reads it for live display.
#[derive(Debug, Default)]
pub struct InputCacheStore {
    values: HashMap<InputAddress, InputValue>,
}

impl InputCacheStore {
    /// Create a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update the cached value for an input address.
    pub fn update(&mut self, address: &InputAddress, value: &InputValue) {
        self.values.insert(address.clone(), value.clone());
    }

    /// Return all cached axis entries as (address, value) pairs.
    ///
    /// Used for axis refresh when the active mode changes: every
    /// cached axis is re-processed through the new mode's pipeline.
    #[must_use]
    pub fn get_all_axis_entries(&self) -> Vec<(InputAddress, f64)> {
        self.values
            .iter()
            .filter_map(|(addr, val)| {
                if let InputValue::Axis { value } = val {
                    Some((addr.clone(), value.value()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Remove all cached values for inputs belonging to `device`.
    pub fn evict_device(&mut self, device: &DeviceId) {
        self.values.retain(|addr, _| addr.device != *device);
    }

    /// Remove all cached values.
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl InputCache for InputCacheStore {
    fn get_button(&self, address: &InputAddress) -> bool {
        self.values
            .get(address)
            .is_some_and(|v| matches!(v, InputValue::Button { pressed: true }))
    }

    fn get_axis(&self, address: &InputAddress) -> f64 {
        self.values.get(address).map_or(0.0, |v| {
            if let InputValue::Axis { value } = v {
                value.value()
            } else {
                0.0
            }
        })
    }

    fn get_hat(&self, address: &InputAddress) -> HatDirection {
        self.values.get(address).map_or(HatDirection::Center, |v| {
            if let InputValue::Hat { direction } = v {
                *direction
            } else {
                HatDirection::Center
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AxisValue, InputId};

    fn axis_address(index: u8) -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index },
        }
    }

    fn button_address(index: u8) -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index },
        }
    }

    fn hat_address(index: u8) -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Hat { index },
        }
    }

    // --- Default values for missing entries ---

    #[test]
    fn get_button_default_is_false() {
        let cache = InputCacheStore::new();
        assert!(!cache.get_button(&button_address(0)));
    }

    #[test]
    fn get_axis_default_is_zero() {
        let cache = InputCacheStore::new();
        assert!((cache.get_axis(&axis_address(0))).abs() < f64::EPSILON);
    }

    #[test]
    fn get_hat_default_is_center() {
        let cache = InputCacheStore::new();
        assert_eq!(cache.get_hat(&hat_address(0)), HatDirection::Center);
    }

    // --- Update and retrieve ---

    #[test]
    fn update_and_get_button_pressed() {
        let mut cache = InputCacheStore::new();
        let addr = button_address(3);
        cache.update(&addr, &InputValue::Button { pressed: true });
        assert!(cache.get_button(&addr));
    }

    #[test]
    fn update_and_get_button_released() {
        let mut cache = InputCacheStore::new();
        let addr = button_address(3);
        cache.update(&addr, &InputValue::Button { pressed: false });
        assert!(!cache.get_button(&addr));
    }

    #[test]
    fn update_and_get_axis() {
        let mut cache = InputCacheStore::new();
        let addr = axis_address(0);
        cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(0.75),
            },
        );
        assert!((cache.get_axis(&addr) - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn update_and_get_hat() {
        let mut cache = InputCacheStore::new();
        let addr = hat_address(0);
        cache.update(
            &addr,
            &InputValue::Hat {
                direction: HatDirection::NE,
            },
        );
        assert_eq!(cache.get_hat(&addr), HatDirection::NE);
    }

    #[test]
    fn update_overwrites_previous_value() {
        let mut cache = InputCacheStore::new();
        let addr = axis_address(0);
        cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );
        cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(-0.3),
            },
        );
        assert!((cache.get_axis(&addr) - (-0.3)).abs() < f64::EPSILON);
    }

    // --- Type mismatch defaults ---

    #[test]
    fn get_button_on_axis_entry_returns_false() {
        let mut cache = InputCacheStore::new();
        let addr = axis_address(0);
        cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(1.0),
            },
        );
        assert!(!cache.get_button(&addr));
    }

    #[test]
    fn get_axis_on_button_entry_returns_zero() {
        let mut cache = InputCacheStore::new();
        let addr = button_address(0);
        cache.update(&addr, &InputValue::Button { pressed: true });
        assert!((cache.get_axis(&addr)).abs() < f64::EPSILON);
    }

    // --- get_all_axis_entries ---

    #[test]
    fn get_all_axis_entries_filters_only_axes() {
        let mut cache = InputCacheStore::new();
        cache.update(
            &axis_address(0),
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );
        cache.update(
            &axis_address(1),
            &InputValue::Axis {
                value: AxisValue::new(-0.3),
            },
        );
        cache.update(&button_address(0), &InputValue::Button { pressed: true });
        cache.update(
            &hat_address(0),
            &InputValue::Hat {
                direction: HatDirection::N,
            },
        );

        let entries = cache.get_all_axis_entries();
        assert_eq!(entries.len(), 2);
        // Both should be axis values
        for (_, val) in &entries {
            assert!(val.abs() <= 1.0);
        }
    }

    #[test]
    fn get_all_axis_entries_empty_cache() {
        let cache = InputCacheStore::new();
        assert!(cache.get_all_axis_entries().is_empty());
    }

    // --- clear ---

    #[test]
    fn clear_removes_all_entries() {
        let mut cache = InputCacheStore::new();
        cache.update(
            &axis_address(0),
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );
        cache.update(&button_address(0), &InputValue::Button { pressed: true });
        cache.clear();
        assert!((cache.get_axis(&axis_address(0))).abs() < f64::EPSILON);
        assert!(!cache.get_button(&button_address(0)));
        assert!(cache.get_all_axis_entries().is_empty());
    }
}
