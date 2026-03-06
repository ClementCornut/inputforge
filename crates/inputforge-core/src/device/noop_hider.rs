// Rust guideline compliant 2026-03-06

//! No-op fallback implementation of [`DeviceHider`].
//!
//! Used when the `HidHide` driver is unavailable (e.g., not installed or
//! running on an unsupported platform). All operations succeed silently
//! and [`NoOpDeviceHider::is_active`] always returns `false`.

use crate::error::Result;
use crate::types::DeviceInfo;

use super::DeviceHider;

/// A [`DeviceHider`] that does nothing.
///
/// Useful as a fallback when the real `HidHide` driver cannot be loaded.
#[derive(Debug, Default)]
pub struct NoOpDeviceHider;

impl DeviceHider for NoOpDeviceHider {
    fn hide_device(&mut self, _device: &DeviceInfo) -> Result<()> {
        Ok(())
    }

    fn unhide_device(&mut self, _device: &DeviceInfo) -> Result<()> {
        Ok(())
    }

    fn is_active(&self) -> bool {
        false
    }

    fn list_hidden_devices(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}
