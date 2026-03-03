// Rust guideline compliant 2026-03-03

use crate::error::Result;
use crate::types::{DeviceId, DeviceInfo, InputEvent};

/// Reads physical input devices (joysticks, pedals, throttles).
///
/// Implementations wrap a platform-specific input library (e.g., SDL3)
/// and normalize events into [`InputEvent`] values.
///
/// # Thread Safety
///
/// This trait intentionally does **not** require `Send`. The primary
/// implementation (`Sdl3Input`) is `!Send` because the underlying
/// SDL3 context must be used from the thread that created it. The
/// [`Engine`](crate::engine::Engine) must be constructed and run on
/// the same thread where the `InputSource` was created.
pub trait InputSource {
    /// List all currently connected physical devices.
    fn enumerate_devices(&self) -> Vec<DeviceInfo>;

    /// Poll for new input events, appending them to `out`.
    ///
    /// Using an output parameter lets callers reuse the allocation buffer
    /// across frames instead of allocating a new `Vec` each time.
    fn poll(&mut self, out: &mut Vec<InputEvent>);

    /// Check whether a specific device is still connected.
    fn is_device_connected(&self, id: &DeviceId) -> bool;

    /// Drain any hotplug events buffered since the last call.
    fn hotplug_events(&mut self) -> Vec<HotplugEvent>;
}

/// Hides physical devices from other applications so only the virtual
/// device is visible (e.g., via `HidHide` on Windows).
///
/// # Thread Safety
///
/// This trait does not require `Send`. The
/// [`Engine`](crate::engine::Engine) owns the implementation and
/// calls it exclusively from the engine thread, so cross-thread
/// access is never needed.
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

    /// Returns the list of currently hidden device instance paths.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying driver query fails.
    fn list_hidden_devices(&self) -> Result<Vec<String>>;
}

/// Device connection or disconnection notification.
#[derive(Debug, Clone)]
pub enum HotplugEvent {
    Connected(DeviceInfo),
    Disconnected(DeviceId),
}
