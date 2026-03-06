// Rust guideline compliant 2026-03-06

//! Cache of the latest values written to virtual vJoy outputs.
//!
//! Mirrors [`super::InputCacheStore`] but stores vJoy output values
//! instead of physical input values. The engine updates this cache
//! after every pipeline execution; the GUI reads it for live display
//! in the input viewer window.

use std::collections::HashMap;

use crate::types::{HatDirection, VJoyAxis};

/// Stores the latest value for every virtual vJoy output.
///
/// Uses separate `HashMap`s for each value type (axis, button, hat)
/// keyed by `(device_id, output_id)` tuples. The engine writes to
/// this cache after pipeline execution; the GUI reads it for display.
#[derive(Debug, Default)]
pub struct OutputCacheStore {
    axes: HashMap<(u8, VJoyAxis), f64>,
    buttons: HashMap<(u8, u8), bool>,
    hats: HashMap<(u8, u8), HatDirection>,
}

impl OutputCacheStore {
    /// Create a new empty output cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update the cached value for a vJoy axis output.
    pub fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) {
        self.axes.insert((device, axis), value);
    }

    /// Insert or update the cached value for a vJoy button output.
    pub fn set_button(&mut self, device: u8, button: u8, pressed: bool) {
        self.buttons.insert((device, button), pressed);
    }

    /// Insert or update the cached value for a vJoy hat output.
    pub fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) {
        self.hats.insert((device, hat), direction);
    }

    /// Return the cached axis value, or `0.0` if not present.
    #[must_use]
    pub fn get_axis(&self, device: u8, axis: VJoyAxis) -> f64 {
        self.axes.get(&(device, axis)).copied().unwrap_or(0.0)
    }

    /// Return the cached button state, or `false` if not present.
    #[must_use]
    pub fn get_button(&self, device: u8, button: u8) -> bool {
        self.buttons
            .get(&(device, button))
            .copied()
            .unwrap_or(false)
    }

    /// Return the cached hat direction, or [`HatDirection::Center`] if not present.
    #[must_use]
    pub fn get_hat(&self, device: u8, hat: u8) -> HatDirection {
        self.hats
            .get(&(device, hat))
            .copied()
            .unwrap_or(HatDirection::Center)
    }

    /// Remove all cached output values.
    pub fn clear(&mut self) {
        self.axes.clear();
        self.buttons.clear();
        self.hats.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Default values for missing entries ---

    #[test]
    fn get_axis_default_is_zero() {
        let cache = OutputCacheStore::new();
        assert!(cache.get_axis(1, VJoyAxis::X).abs() < f64::EPSILON);
    }

    #[test]
    fn get_button_default_is_false() {
        let cache = OutputCacheStore::new();
        assert!(!cache.get_button(1, 0));
    }

    #[test]
    fn get_hat_default_is_center() {
        let cache = OutputCacheStore::new();
        assert_eq!(cache.get_hat(1, 0), HatDirection::Center);
    }

    // --- Set and retrieve ---

    #[test]
    fn set_and_get_axis() {
        let mut cache = OutputCacheStore::new();
        cache.set_axis(1, VJoyAxis::X, 0.75);
        assert!((cache.get_axis(1, VJoyAxis::X) - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn set_and_get_button_pressed() {
        let mut cache = OutputCacheStore::new();
        cache.set_button(1, 3, true);
        assert!(cache.get_button(1, 3));
    }

    #[test]
    fn set_and_get_button_released() {
        let mut cache = OutputCacheStore::new();
        cache.set_button(1, 3, false);
        assert!(!cache.get_button(1, 3));
    }

    #[test]
    fn set_and_get_hat() {
        let mut cache = OutputCacheStore::new();
        cache.set_hat(1, 0, HatDirection::NE);
        assert_eq!(cache.get_hat(1, 0), HatDirection::NE);
    }

    // --- Upsert overwrites previous value ---

    #[test]
    fn set_axis_overwrites_previous() {
        let mut cache = OutputCacheStore::new();
        cache.set_axis(1, VJoyAxis::Y, 0.5);
        cache.set_axis(1, VJoyAxis::Y, -0.3);
        assert!((cache.get_axis(1, VJoyAxis::Y) - (-0.3)).abs() < f64::EPSILON);
    }

    #[test]
    fn set_button_overwrites_previous() {
        let mut cache = OutputCacheStore::new();
        cache.set_button(1, 0, true);
        cache.set_button(1, 0, false);
        assert!(!cache.get_button(1, 0));
    }

    #[test]
    fn set_hat_overwrites_previous() {
        let mut cache = OutputCacheStore::new();
        cache.set_hat(1, 0, HatDirection::N);
        cache.set_hat(1, 0, HatDirection::SW);
        assert_eq!(cache.get_hat(1, 0), HatDirection::SW);
    }

    // --- Multiple devices are independent ---

    #[test]
    fn different_devices_are_independent() {
        let mut cache = OutputCacheStore::new();
        cache.set_axis(1, VJoyAxis::X, 0.5);
        cache.set_axis(2, VJoyAxis::X, -0.8);
        assert!((cache.get_axis(1, VJoyAxis::X) - 0.5).abs() < f64::EPSILON);
        assert!((cache.get_axis(2, VJoyAxis::X) - (-0.8)).abs() < f64::EPSILON);
    }

    #[test]
    fn different_axes_are_independent() {
        let mut cache = OutputCacheStore::new();
        cache.set_axis(1, VJoyAxis::X, 0.5);
        cache.set_axis(1, VJoyAxis::Y, -0.3);
        assert!((cache.get_axis(1, VJoyAxis::X) - 0.5).abs() < f64::EPSILON);
        assert!((cache.get_axis(1, VJoyAxis::Y) - (-0.3)).abs() < f64::EPSILON);
    }

    // --- Clear ---

    #[test]
    fn clear_removes_all_entries() {
        let mut cache = OutputCacheStore::new();
        cache.set_axis(1, VJoyAxis::X, 0.5);
        cache.set_button(1, 0, true);
        cache.set_hat(1, 0, HatDirection::N);
        cache.clear();
        assert!(cache.get_axis(1, VJoyAxis::X).abs() < f64::EPSILON);
        assert!(!cache.get_button(1, 0));
        assert_eq!(cache.get_hat(1, 0), HatDirection::Center);
    }

    #[test]
    fn clear_on_empty_cache_is_noop() {
        let mut cache = OutputCacheStore::new();
        cache.clear();
        assert!(cache.get_axis(1, VJoyAxis::X).abs() < f64::EPSILON);
    }

    // --- Debug and Default ---

    #[test]
    fn debug_format_contains_struct_name() {
        let cache = OutputCacheStore::new();
        let debug = format!("{cache:?}");
        assert!(debug.contains("OutputCacheStore"));
    }

    #[test]
    fn default_trait_creates_empty_cache() {
        let cache = OutputCacheStore::default();
        assert!(cache.get_axis(1, VJoyAxis::X).abs() < f64::EPSILON);
        assert!(!cache.get_button(1, 0));
        assert_eq!(cache.get_hat(1, 0), HatDirection::Center);
    }
}
