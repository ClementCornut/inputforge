// Rust guideline compliant 2026-03-03

use crate::error::Result;
use crate::types::{HatDirection, VJoyAxis, VirtualDeviceConfig};

/// Trait for sending output to virtual devices.
///
/// Implementations write axis, button, and hat values to virtual
/// joystick devices (e.g., vJoy).
pub trait OutputSink {
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
}
