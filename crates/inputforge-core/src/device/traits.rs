// Rust guideline compliant 2026-03-03

use super::InputUpdate;
use crate::error::Result;
use crate::profile::controllers::DeviceBinding;
use crate::types::{DeviceDiagnostics, DeviceId, DeviceInfo, InputId};

/// Reads physical input devices (joysticks, pedals, throttles).
///
/// Implementations wrap a platform-specific input library (e.g., SDL3)
/// and normalize events into [`crate::types::InputEvent`] values.
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
    ///
    /// # Errors
    ///
    /// Returns an error when input ownership or streaming fails.
    fn poll(&mut self, out: &mut Vec<InputUpdate>) -> Result<()>;

    /// Whether the backend can acquire exclusive physical access.
    fn supports_exclusive(&self) -> bool {
        false
    }
    /// Install metadata only.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid configuration.
    fn configure(&mut self, _bindings: &[DeviceBinding]) -> Result<()> {
        Ok(())
    }
    /// Acquire a complete selection.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable input.
    fn acquire(&mut self, _ids: &[DeviceId]) -> Result<()> {
        Ok(())
    }
    /// Release exclusive ownership while preserving passive monitoring.
    ///
    /// # Errors
    ///
    /// Returns an error if exclusive ownership could not be released cleanly.
    fn release(&mut self) -> Result<()> {
        Ok(())
    }
    /// Refresh failed discovery explicitly.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable inventory.
    fn refresh(&mut self) -> Result<()> {
        Ok(())
    }
    /// Discover a frozen native table without grabbing.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid capabilities.
    fn binding_table(&self, _id: &DeviceId) -> Result<Option<DeviceBinding>> {
        Ok(None)
    }

    /// Request a fresh native resting sample for explicit axis redetection.
    /// # Errors
    /// Returns an error when the device is unavailable.
    fn request_axis_sample(&mut self, _device: &DeviceId, _axis: u8) -> Result<()> {
        Ok(())
    }

    /// Confirm a user-selected native control after a positional layout changed.
    /// # Errors
    /// Rejects controls that the adapter cannot currently identify.
    fn confirm_binding(&mut self, _device: &DeviceId, _input: &InputId) -> Result<DeviceBinding> {
        Err(crate::error::EngineError::InvalidConfig {
            reason: "This controller cannot confirm a native input binding".into(),
        })
    }

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
/// This trait does not require `Send` and makes no cross-thread access
/// guarantees. Callers must keep an implementation on its required thread when
/// the underlying platform API is thread-affine.
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
    Connected {
        info: DeviceInfo,
        diagnostics: DeviceDiagnostics,
    },
    Disconnected(DeviceId),
}
