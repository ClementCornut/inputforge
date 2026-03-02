// Rust guideline compliant 2026-03-02

use crate::error::Result;
use crate::types::{DeviceId, DeviceInfo, InputEvent};

/// Reads physical input devices (joysticks, pedals, throttles).
///
/// Implementations wrap a platform-specific input library (e.g., SDL3)
/// and normalize events into [`InputEvent`] values.
pub trait InputSource {
    /// List all currently connected physical devices.
    fn enumerate_devices(&self) -> Vec<DeviceInfo>;

    /// Poll for new input events. Returns all events since the last call.
    fn poll(&mut self) -> Vec<InputEvent>;

    /// Check whether a specific device is still connected.
    fn is_device_connected(&self, id: &DeviceId) -> bool;

    /// Drain any hotplug events buffered since the last call.
    fn hotplug_events(&mut self) -> Vec<HotplugEvent>;
}

/// Hides physical devices from other applications so only the virtual
/// device is visible (e.g., via `HidHide` on Windows).
pub trait DeviceHider {
    /// Add a device to the hidden-device list.
    ///
    /// # Errors
    ///
    /// Returns an error if the hiding driver is unavailable or the
    /// device path cannot be resolved.
    fn hide_device(&mut self, device: &DeviceInfo) -> Result<()>;

    /// Remove a device from the hidden-device list.
    ///
    /// # Errors
    ///
    /// Returns an error if the hiding driver is unavailable or the
    /// device path cannot be resolved.
    fn unhide_device(&mut self, device: &DeviceInfo) -> Result<()>;

    /// Check whether the hiding driver is currently active.
    fn is_active(&self) -> bool;
}

/// Device connection or disconnection notification.
#[derive(Debug, Clone)]
pub enum HotplugEvent {
    Connected(DeviceInfo),
    Disconnected(DeviceId),
}
