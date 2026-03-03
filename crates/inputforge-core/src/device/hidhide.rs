// Rust guideline compliant 2026-03-03

use std::fmt;

use windows::Win32::Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::core::w;

use crate::error::{EngineError, Result};
use crate::types::DeviceInfo;

use super::traits::DeviceHider;

// HidHide IOCTL codes computed via CTL_CODE(0x8001, fn, METHOD_BUFFERED, FILE_READ_DATA).
// Source: https://github.com/nefarius/HidHide/blob/master/HidHide/src/Logic.h

/// Get the current device instance blacklist.
const IOCTL_GET_BLACKLIST: u32 = 0x8001_6008;

/// Set the device instance blacklist.
const IOCTL_SET_BLACKLIST: u32 = 0x8001_600C;

/// Get the current application whitelist.
const IOCTL_GET_WHITELIST: u32 = 0x8001_6000;

/// Set the application whitelist.
const IOCTL_SET_WHITELIST: u32 = 0x8001_6004;

/// Get the current active/enabled state (1 byte: 1=active, 0=inactive).
const IOCTL_GET_ACTIVE: u32 = 0x8001_6010;

/// Set the active/enabled state (1 byte: 1=active, 0=inactive).
const IOCTL_SET_ACTIVE: u32 = 0x8001_6014;

/// Initial buffer size for reading blacklist/whitelist strings.
///
/// `HidHide` returns double-null-terminated UTF-16LE strings. 4 KiB
/// is sufficient for typical installations with a few devices.
const INITIAL_BUFFER_SIZE: usize = 4096;

/// Maximum buffer size for multi-string reads (256 KiB).
///
/// Caps the retry loop in [`read_multi_string`] to prevent unbounded
/// allocation when the IOCTL repeatedly reports insufficient buffer.
const MAX_BUFFER_SIZE: usize = 256 * 1024;

/// Manager for the `HidHide` device-hiding driver.
///
/// Communicates with the `HidHide` control device via IOCTL to maintain
/// a blacklist of hidden device instance paths.
pub struct HidHideManager {
    handle: HANDLE,
    active: bool,
}

impl fmt::Debug for HidHideManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HidHideManager")
            .field("active", &self.active)
            .finish_non_exhaustive()
    }
}

impl HidHideManager {
    /// Open a connection to the `HidHide` control device.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HidHide`] if the `HidHide` driver is not
    /// installed or the control device cannot be opened.
    #[expect(unsafe_code, reason = "CreateFileW is a Win32 FFI call")]
    pub fn new() -> Result<Self> {
        // SAFETY: Opening a device handle with valid parameters. The wide
        // string literal is null-terminated by the `w!` macro.
        let handle = unsafe {
            CreateFileW(
                w!("\\\\.\\HidHide"),
                0, // No specific access rights needed for IOCTL
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .map_err(|e| EngineError::HidHide(format!("failed to open control device: {e}")))?;

        let active = read_active(handle)?;

        Ok(Self { handle, active })
    }

    /// Add our application to the `HidHide` whitelist so we can still
    /// see hidden devices.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HidHide`] if the IOCTL call fails.
    pub fn whitelist_self(&mut self) -> Result<()> {
        let exe_path = std::env::current_exe()
            .map_err(|e| EngineError::HidHide(format!("failed to get exe path: {e}")))?;
        let exe_str = exe_path.to_string_lossy();

        let mut whitelist = read_multi_string(self.handle, IOCTL_GET_WHITELIST)?;
        if !whitelist.iter().any(|p| p == exe_str.as_ref()) {
            whitelist.push(exe_str.into_owned());
            write_multi_string(self.handle, IOCTL_SET_WHITELIST, &whitelist)?;
        }
        Ok(())
    }

    /// Set the `HidHide` active/enabled state.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HidHide`] if the IOCTL call fails.
    pub fn set_active(&mut self, active: bool) -> Result<()> {
        write_active(self.handle, active)?;
        self.active = active;
        Ok(())
    }
}

impl DeviceHider for HidHideManager {
    fn hide_device(&mut self, device: &DeviceInfo) -> Result<()> {
        let instance_path = device.instance_path.as_deref().ok_or_else(|| {
            EngineError::HidHide(format!(
                "device '{}' has no instance path for hiding",
                device.name
            ))
        })?;
        let mut blacklist = read_multi_string(self.handle, IOCTL_GET_BLACKLIST)?;
        if !blacklist.iter().any(|p| p == instance_path) {
            blacklist.push(instance_path.to_owned());
            write_multi_string(self.handle, IOCTL_SET_BLACKLIST, &blacklist)?;
        }
        Ok(())
    }

    fn unhide_device(&mut self, device: &DeviceInfo) -> Result<()> {
        let instance_path = device.instance_path.as_deref().ok_or_else(|| {
            EngineError::HidHide(format!(
                "device '{}' has no instance path for unhiding",
                device.name
            ))
        })?;
        let mut blacklist = read_multi_string(self.handle, IOCTL_GET_BLACKLIST)?;
        let before = blacklist.len();
        blacklist.retain(|p| p != instance_path);
        if blacklist.len() != before {
            write_multi_string(self.handle, IOCTL_SET_BLACKLIST, &blacklist)?;
        }
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for HidHideManager {
    #[expect(unsafe_code, reason = "CloseHandle is a Win32 FFI call")]
    fn drop(&mut self) {
        // SAFETY: `self.handle` was obtained from `CreateFileW` and has not
        // been closed yet. Closing it here prevents resource leaks.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

// ---------------------------------------------------------------------------
// IOCTL helpers
// ---------------------------------------------------------------------------

/// Read the `HidHide` active state (single boolean byte).
#[expect(unsafe_code, reason = "DeviceIoControl is a Win32 FFI call")]
fn read_active(handle: HANDLE) -> Result<bool> {
    let mut buf: [u8; 1] = [0];
    let mut returned = 0u32;

    // SAFETY: `buf` is a valid 1-byte buffer. `DeviceIoControl` writes at
    // most 1 byte and sets `returned` to the actual count.
    unsafe {
        DeviceIoControl(
            handle,
            IOCTL_GET_ACTIVE,
            None,
            0,
            Some(buf.as_mut_ptr().cast()),
            u32::try_from(buf.len()).unwrap_or(1),
            Some(&raw mut returned),
            None,
        )
    }
    .map_err(|e| EngineError::HidHide(format!("IOCTL_GET_ACTIVE failed: {e}")))?;

    Ok(buf[0] != 0)
}

/// Write the `HidHide` active state.
#[expect(unsafe_code, reason = "DeviceIoControl is a Win32 FFI call")]
fn write_active(handle: HANDLE, active: bool) -> Result<()> {
    let buf: [u8; 1] = [u8::from(active)];
    let mut returned = 0u32;

    // SAFETY: `buf` is a valid 1-byte buffer containing the new state.
    unsafe {
        DeviceIoControl(
            handle,
            IOCTL_SET_ACTIVE,
            Some(buf.as_ptr().cast()),
            u32::try_from(buf.len()).unwrap_or(1),
            None,
            0,
            Some(&raw mut returned),
            None,
        )
    }
    .map_err(|e| EngineError::HidHide(format!("IOCTL_SET_ACTIVE failed: {e}")))?;

    Ok(())
}

/// Read a double-null-terminated UTF-16LE multi-string from an IOCTL.
///
/// Retries with a doubled buffer if `ERROR_INSUFFICIENT_BUFFER` is returned,
/// up to [`MAX_BUFFER_SIZE`].
#[expect(unsafe_code, reason = "DeviceIoControl is a Win32 FFI call")]
fn read_multi_string(handle: HANDLE, ioctl: u32) -> Result<Vec<String>> {
    let mut buf_size = INITIAL_BUFFER_SIZE;

    loop {
        let mut buf: Vec<u16> = vec![0; buf_size / 2];
        let mut returned = 0u32;

        #[expect(
            clippy::cast_possible_truncation,
            reason = "buf_size is bounded by MAX_BUFFER_SIZE (256 KiB) which fits in u32"
        )]
        let byte_len = (buf.len() * 2) as u32;

        // SAFETY: `buf` is a properly aligned u16 buffer. `DeviceIoControl`
        // writes at most `byte_len` bytes.
        let result = unsafe {
            DeviceIoControl(
                handle,
                ioctl,
                None,
                0,
                Some(buf.as_mut_ptr().cast()),
                byte_len,
                Some(&raw mut returned),
                None,
            )
        };

        match result {
            Ok(()) => {
                let u16_count = returned as usize / 2;
                buf.truncate(u16_count);
                return Ok(decode_multi_string(&buf));
            }
            Err(e) => {
                if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() && buf_size < MAX_BUFFER_SIZE
                {
                    buf_size = buf_size.saturating_mul(2).min(MAX_BUFFER_SIZE);
                    tracing::debug!(
                        buf_size,
                        "IOCTL buffer too small, retrying with larger buffer"
                    );
                    continue;
                }
                return Err(EngineError::HidHide(format!(
                    "IOCTL read multi-string failed: {e}"
                )));
            }
        }
    }
}

/// Write a double-null-terminated UTF-16LE multi-string via an IOCTL.
#[expect(unsafe_code, reason = "DeviceIoControl is a Win32 FFI call")]
fn write_multi_string(handle: HANDLE, ioctl: u32, entries: &[String]) -> Result<()> {
    let buf = encode_multi_string(entries);
    let mut returned = 0u32;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "multi-string buffer size fits in u32"
    )]
    let byte_len = (buf.len() * 2) as u32;

    // SAFETY: `buf` is a properly encoded double-null-terminated u16 buffer.
    unsafe {
        DeviceIoControl(
            handle,
            ioctl,
            Some(buf.as_ptr().cast()),
            byte_len,
            None,
            0,
            Some(&raw mut returned),
            None,
        )
    }
    .map_err(|e| EngineError::HidHide(format!("IOCTL write multi-string failed: {e}")))?;

    Ok(())
}

/// Decode a double-null-terminated UTF-16LE buffer into a list of strings.
///
/// Each string is null-terminated, with an extra trailing null marking the
/// end of the list.
fn decode_multi_string(buf: &[u16]) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0;

    for (i, &ch) in buf.iter().enumerate() {
        if ch == 0 {
            if i > start {
                result.push(String::from_utf16_lossy(&buf[start..i]));
            } else {
                // Double null: end of list.
                break;
            }
            start = i + 1;
        }
    }

    result
}

/// Encode a list of strings into a double-null-terminated UTF-16LE buffer.
fn encode_multi_string(entries: &[String]) -> Vec<u16> {
    let mut buf = Vec::new();
    for entry in entries {
        buf.extend(entry.encode_utf16());
        buf.push(0); // null terminator for this entry
    }
    buf.push(0); // final null for double-null termination
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compute a `CTL_CODE` value from its components.
    ///
    /// Formula: `(device_type << 16) | (access << 14) | (function << 2) | method`
    const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
        (device_type << 16) | (access << 14) | (function << 2) | method
    }

    // CTL_CODE parameters for HidHide:
    // IoControlDeviceType = 32769 (0x8001)
    // METHOD_BUFFERED = 0
    // FILE_READ_DATA = 1
    const DEVICE_TYPE: u32 = 0x8001;
    const METHOD_BUFFERED: u32 = 0;
    const FILE_READ_DATA: u32 = 1;

    #[test]
    fn ioctl_get_whitelist_value() {
        assert_eq!(
            ctl_code(DEVICE_TYPE, 2048, METHOD_BUFFERED, FILE_READ_DATA),
            IOCTL_GET_WHITELIST
        );
    }

    #[test]
    fn ioctl_set_whitelist_value() {
        assert_eq!(
            ctl_code(DEVICE_TYPE, 2049, METHOD_BUFFERED, FILE_READ_DATA),
            IOCTL_SET_WHITELIST
        );
    }

    #[test]
    fn ioctl_get_blacklist_value() {
        assert_eq!(
            ctl_code(DEVICE_TYPE, 2050, METHOD_BUFFERED, FILE_READ_DATA),
            IOCTL_GET_BLACKLIST
        );
    }

    #[test]
    fn ioctl_set_blacklist_value() {
        assert_eq!(
            ctl_code(DEVICE_TYPE, 2051, METHOD_BUFFERED, FILE_READ_DATA),
            IOCTL_SET_BLACKLIST
        );
    }

    #[test]
    fn ioctl_get_active_value() {
        assert_eq!(
            ctl_code(DEVICE_TYPE, 2052, METHOD_BUFFERED, FILE_READ_DATA),
            IOCTL_GET_ACTIVE
        );
    }

    #[test]
    fn ioctl_set_active_value() {
        assert_eq!(
            ctl_code(DEVICE_TYPE, 2053, METHOD_BUFFERED, FILE_READ_DATA),
            IOCTL_SET_ACTIVE
        );
    }

    #[test]
    fn encode_decode_multi_string_roundtrip() {
        let entries = vec![
            "HID\\VID_045E&PID_02FF".to_owned(),
            "USB\\VID_1234&PID_5678".to_owned(),
        ];
        let encoded = encode_multi_string(&entries);
        let decoded = decode_multi_string(&encoded);
        assert_eq!(decoded, entries);
    }

    #[test]
    fn encode_empty_list() {
        let entries: Vec<String> = vec![];
        let encoded = encode_multi_string(&entries);
        // Should be just a single null (double-null with the implicit end).
        assert_eq!(encoded, vec![0]);
    }

    #[test]
    fn decode_empty_buffer() {
        let buf: Vec<u16> = vec![0, 0];
        let decoded = decode_multi_string(&buf);
        assert!(decoded.is_empty());
    }

    #[test]
    fn decode_single_entry() {
        let entry = "TEST\\DEVICE";
        let mut buf: Vec<u16> = entry.encode_utf16().collect();
        buf.push(0); // null terminator
        buf.push(0); // double null
        let decoded = decode_multi_string(&buf);
        assert_eq!(decoded, vec!["TEST\\DEVICE"]);
    }

    #[test]
    fn encode_preserves_backslashes_and_ampersands() {
        let entries = vec!["HID\\VID_045E&PID_02FF&IG_00#7&51dab6b&0&0000".to_owned()];
        let encoded = encode_multi_string(&entries);
        let decoded = decode_multi_string(&encoded);
        assert_eq!(decoded, entries);
    }
}
