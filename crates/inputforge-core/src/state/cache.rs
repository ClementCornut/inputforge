// Rust guideline compliant 2026-04-30

use indexmap::IndexMap;

use crate::pipeline::InputCache;
use crate::types::{AxisPolarity, DeviceId, HatDirection, InputAddress, InputValue};

/// Stores the latest value for every physical input.
///
/// Implements [`InputCache`] so it can be passed directly into
/// pipeline execution. The engine updates this cache on every
/// input event; the GUI reads it for live display.
///
/// **Iteration order guarantee**: the internal store is an [`IndexMap`],
/// which preserves insertion order. This makes [`InputCacheStore::clone_compact`]
/// stable and deterministic across calls, a requirement for the
/// live-capture tied-axis tiebreak logic in `patterns::live_capture`.
#[derive(Debug, Default)]
pub struct InputCacheStore {
    values: IndexMap<InputAddress, InputValue>,
}

/// One entry in an [`InputCacheStore`] snapshot. Used by GUI consumers
/// (notably the live-capture primitive) that need to compare current
/// state against an earlier baseline without holding any read lock.
///
/// Derives [`PartialEq`] so polling effects can do `prev != next`
/// equality checks without manually comparing fields.
#[derive(Debug, Clone, PartialEq)]
pub struct InputCacheEntry {
    pub address: InputAddress,
    pub value: InputValue,
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

    /// Return all cached axis entries as `(address, value, polarity)`
    /// triples.
    ///
    /// Used for axis refresh when the active mode changes: every
    /// cached axis is re-processed through the new mode's pipeline.
    #[must_use]
    pub fn get_all_axis_entries(&self) -> Vec<(InputAddress, f64, AxisPolarity)> {
        self.values
            .iter()
            .filter_map(|(addr, val)| {
                if let InputValue::Axis { value, polarity } = val {
                    Some((addr.clone(), value.value(), *polarity))
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

    /// Snapshot every cached `(address, value)` pair into an owned [`Vec`].
    ///
    /// # Iteration order
    ///
    /// Order is **stable and deterministic** across calls because the
    /// internal store is an [`IndexMap`], which preserves insertion order.
    /// The live-capture tied-axis tiebreak
    /// (`patterns::live_capture::machine::pick_winner`) relies on
    /// first-encountered order being well-defined: when two axes cross
    /// deadband simultaneously with identical absolute deltas, the first
    /// one in this iteration order wins.
    ///
    /// The return value is fully owned, so the caller can drop the
    /// underlying lock guard immediately after this call returns.
    #[must_use]
    pub fn clone_compact(&self) -> Vec<InputCacheEntry> {
        self.values
            .iter()
            .map(|(addr, val)| InputCacheEntry {
                address: addr.clone(),
                value: val.clone(),
            })
            .collect()
    }
}

impl InputCache for InputCacheStore {
    fn get_button(&self, address: &InputAddress) -> bool {
        self.values
            .get(address)
            .is_some_and(|v| matches!(v, InputValue::Button { pressed: true }))
    }

    fn get_axis(&self, address: &InputAddress) -> (f64, AxisPolarity) {
        self.values
            .get(address)
            .map_or((0.0, AxisPolarity::default()), |v| {
                if let InputValue::Axis { value, polarity } = v {
                    (value.value(), *polarity)
                } else {
                    (0.0, AxisPolarity::default())
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
        let (value, polarity) = cache.get_axis(&axis_address(0));
        assert!(value.abs() < f64::EPSILON);
        assert_eq!(polarity, AxisPolarity::Bipolar);
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
                polarity: AxisPolarity::Bipolar,
            },
        );
        let (value, polarity) = cache.get_axis(&addr);
        assert!((value - 0.75).abs() < f64::EPSILON);
        assert_eq!(polarity, AxisPolarity::Bipolar);
    }

    #[test]
    fn update_and_get_axis_unipolar_round_trips_polarity() {
        let mut cache = InputCacheStore::new();
        let addr = axis_address(0);
        cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(-1.0),
                polarity: AxisPolarity::Unipolar,
            },
        );
        let (value, polarity) = cache.get_axis(&addr);
        assert!((value - (-1.0)).abs() < f64::EPSILON);
        assert_eq!(polarity, AxisPolarity::Unipolar);
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
                polarity: AxisPolarity::Bipolar,
            },
        );
        cache.update(
            &addr,
            &InputValue::Axis {
                value: AxisValue::new(-0.3),
                polarity: AxisPolarity::Bipolar,
            },
        );
        let (value, _) = cache.get_axis(&addr);
        assert!((value - (-0.3)).abs() < f64::EPSILON);
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
                polarity: AxisPolarity::Bipolar,
            },
        );
        assert!(!cache.get_button(&addr));
    }

    #[test]
    fn get_axis_on_button_entry_returns_zero() {
        let mut cache = InputCacheStore::new();
        let addr = button_address(0);
        cache.update(&addr, &InputValue::Button { pressed: true });
        let (value, polarity) = cache.get_axis(&addr);
        assert!(value.abs() < f64::EPSILON);
        assert_eq!(polarity, AxisPolarity::Bipolar);
    }

    // --- get_all_axis_entries ---

    #[test]
    fn get_all_axis_entries_filters_only_axes() {
        let mut cache = InputCacheStore::new();
        cache.update(
            &axis_address(0),
            &InputValue::Axis {
                value: AxisValue::new(0.5),
                polarity: AxisPolarity::Bipolar,
            },
        );
        cache.update(
            &axis_address(1),
            &InputValue::Axis {
                value: AxisValue::new(-1.0),
                polarity: AxisPolarity::Unipolar,
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
        for (_, value, _) in &entries {
            assert!(value.abs() <= 1.0);
        }
        // Polarity round-trips through get_all_axis_entries.
        let polarities: Vec<AxisPolarity> = entries.iter().map(|(_, _, p)| *p).collect();
        assert!(polarities.contains(&AxisPolarity::Bipolar));
        assert!(polarities.contains(&AxisPolarity::Unipolar));
    }

    #[test]
    fn get_all_axis_entries_empty_cache() {
        let cache = InputCacheStore::new();
        assert!(cache.get_all_axis_entries().is_empty());
    }

    // --- clone_compact ---

    #[test]
    fn clone_compact_returns_all_entries_with_address_and_value() {
        let mut cache = InputCacheStore::new();
        cache.update(
            &axis_address(0),
            &InputValue::Axis {
                value: AxisValue::new(0.5),
                polarity: AxisPolarity::Bipolar,
            },
        );
        cache.update(&button_address(1), &InputValue::Button { pressed: true });
        cache.update(
            &hat_address(0),
            &InputValue::Hat {
                direction: HatDirection::N,
            },
        );

        let entries = cache.clone_compact();
        assert_eq!(entries.len(), 3, "all three entries should be present");

        let axis_entry = entries
            .iter()
            .find(|e| e.address == axis_address(0))
            .unwrap();
        match &axis_entry.value {
            InputValue::Axis { value, .. } => {
                assert!((value.value() - 0.5).abs() < f64::EPSILON);
            }
            other => panic!("expected Axis variant, got {other:?}"),
        }

        let button_entry = entries
            .iter()
            .find(|e| e.address == button_address(1))
            .unwrap();
        assert!(matches!(
            button_entry.value,
            InputValue::Button { pressed: true }
        ));

        let hat_entry = entries
            .iter()
            .find(|e| e.address == hat_address(0))
            .unwrap();
        assert!(matches!(
            hat_entry.value,
            InputValue::Hat {
                direction: HatDirection::N,
            }
        ));
    }

    #[test]
    fn clone_compact_empty_cache_returns_empty_vec() {
        let cache = InputCacheStore::new();
        assert!(cache.clone_compact().is_empty());
    }

    #[test]
    fn clone_compact_does_not_mutate_cache() {
        let mut cache = InputCacheStore::new();
        cache.update(&button_address(0), &InputValue::Button { pressed: true });

        let _ = cache.clone_compact();
        let _ = cache.clone_compact();

        assert!(cache.get_button(&button_address(0)));
    }

    // --- clear ---

    #[test]
    fn clear_removes_all_entries() {
        let mut cache = InputCacheStore::new();
        cache.update(
            &axis_address(0),
            &InputValue::Axis {
                value: AxisValue::new(0.5),
                polarity: AxisPolarity::Bipolar,
            },
        );
        cache.update(&button_address(0), &InputValue::Button { pressed: true });
        cache.clear();
        let (value, _) = cache.get_axis(&axis_address(0));
        assert!(value.abs() < f64::EPSILON);
        assert!(!cache.get_button(&button_address(0)));
        assert!(cache.get_all_axis_entries().is_empty());
    }
}
