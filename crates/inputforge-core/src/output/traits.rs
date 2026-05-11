// Rust guideline compliant 2026-03-03

use crate::error::Result;
pub use crate::types::VirtualDeviceConfig;

use crate::action::MouseTarget;
use crate::types::{HatDirection, KeyCombo, VJoyAxis};

/// Trait for sending output to virtual devices.
///
/// Implementations write axis, button, and hat values to virtual
/// joystick devices (e.g., vJoy).
pub trait OutputSink: Send {
    /// Create and acquire a virtual device from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is unavailable or already in use.
    fn create_device(&mut self, config: &VirtualDeviceConfig) -> Result<()>;

    /// Set an axis value on a virtual device.
    ///
    /// `value` is in the normalized range \[-1.0, 1.0\].
    ///
    /// # Errors
    ///
    /// Returns an error if the device or axis is not available.
    fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()>;

    /// Set a button state on a virtual device.
    ///
    /// # Errors
    ///
    /// Returns an error if the device or button is not available.
    fn set_button(&mut self, device: u8, button: u8, pressed: bool) -> Result<()>;

    /// Set a hat switch direction on a virtual device.
    ///
    /// # Errors
    ///
    /// Returns an error if the device or hat is not available.
    fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) -> Result<()>;

    /// Release a virtual device, resetting its state.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not currently acquired.
    fn release_device(&mut self, device: u8) -> Result<()>;

    /// Write all pending state changes to the hardware.
    ///
    /// Implementations that batch state changes should flush all dirty
    /// device states in this method. The default no-op implementation is
    /// suitable for sinks that write immediately (e.g., mocks).
    ///
    /// # Errors
    ///
    /// Returns an error if any device state update fails.
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    /// Return configurations for all available virtual devices.
    ///
    /// Implementations should probe the underlying driver and return one
    /// [`VirtualDeviceConfig`] per discovered device. The default returns
    /// an empty list (suitable for mocks or when no virtual driver exists).
    fn list_devices(&self) -> Vec<VirtualDeviceConfig> {
        Vec::new()
    }
}

/// Trait for keyboard output sinks.
///
/// Separated from [`OutputSink`] because keyboard output operates on
/// key combinations rather than virtual device axes/buttons.
pub trait KeyboardSink: Send {
    /// Press the given key combination.
    ///
    /// # Errors
    ///
    /// Returns an error if the key injection fails.
    fn key_down(&mut self, combo: &KeyCombo) -> Result<()>;

    /// Release the given key combination.
    ///
    /// # Errors
    ///
    /// Returns an error if the key injection fails.
    fn key_up(&mut self, combo: &KeyCombo) -> Result<()>;

    /// Press and release the given key combination.
    ///
    /// # Errors
    ///
    /// Returns an error if either key injection fails.
    fn pulse_key(&mut self, combo: &KeyCombo) -> Result<()> {
        self.key_down(combo)?;
        self.key_up(combo)
    }
}

/// Trait for mouse output sinks.
///
/// Separated from [`OutputSink`] because mouse output operates on
/// mouse targets rather than virtual device axes/buttons.
pub trait MouseSink: Send {
    /// Press the given mouse button target.
    ///
    /// # Errors
    ///
    /// Returns an error if the mouse injection fails.
    fn button_down(&mut self, target: MouseTarget) -> Result<()>;

    /// Release the given mouse button target.
    ///
    /// # Errors
    ///
    /// Returns an error if the mouse injection fails.
    fn button_up(&mut self, target: MouseTarget) -> Result<()>;

    /// Press and release the given mouse button target.
    ///
    /// # Errors
    ///
    /// Returns an error if either mouse injection fails.
    fn pulse_button(&mut self, target: MouseTarget) -> Result<()> {
        self.button_down(target)?;
        self.button_up(target)
    }

    /// Scroll the given mouse wheel target.
    ///
    /// # Errors
    ///
    /// Returns an error if the mouse injection fails.
    fn wheel(&mut self, target: MouseTarget) -> Result<()>;
}
