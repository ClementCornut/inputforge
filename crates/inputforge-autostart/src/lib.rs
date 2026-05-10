//! Autostart manager for `InputForge`: writes OS-level launch-at-sign-in state
//! (HKCU registry on Windows, `~/.config/autostart/*.desktop` on Linux).
//!
//! See `docs/superpowers/specs/2026-05-10-f16-startup-preferences-design.md`.

mod error;
#[cfg(target_os = "linux")]
mod linux;
mod noop;
#[cfg(target_os = "windows")]
mod windows;

pub use error::AutostartError;

#[cfg(feature = "mock")]
pub mod mock;

/// Platform-agnostic interface for the OS autostart store.
///
/// Implementations write to HKCU\...\Run on Windows and to
/// `~/.config/autostart/*.desktop` on Linux. The trait is intentionally
/// *not* `Send`/`Sync`: the engine owns it on its single thread and never
/// shares the instance across threads.
///
/// `args` is passed at call time so the engine decides whether to include
/// `--start-minimized`; concrete impls are dumb about that flag.
pub trait AutostartManager {
    /// Read the OS autostart state for this app.
    ///
    /// # Errors
    ///
    /// Returns an [`AutostartError`] when the backend cannot read the
    /// registry / desktop file (permissions, IO, malformed entry).
    fn is_enabled(&self) -> Result<bool, AutostartError>;

    /// Enable or disable the OS autostart entry. When enabling, `args` is
    /// the argv tail registered with the entry (e.g., `&["--start-minimized"]`).
    /// When disabling, `args` is ignored.
    ///
    /// # Errors
    ///
    /// Returns an [`AutostartError`] when the backend rejects the write
    /// (permissions, IO, registry denial).
    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError>;
}

/// Construct the platform-appropriate autostart manager, or a `NoOpAutostart`
/// fallback when `std::env::current_exe()` fails.
///
/// The fallback's `is_enabled()` returns `Ok(false)` and `set_enabled()`
/// returns [`AutostartError::NotSupported`], so the engine and UI degrade
/// gracefully (the toggle stays off; dispatch surfaces a warning toast).
#[must_use]
pub fn new_for_current_platform() -> Box<dyn AutostartManager> {
    #[cfg(target_os = "windows")]
    {
        match windows::WindowsAutostart::new() {
            Ok(w) => return Box::new(w),
            Err(e) => {
                tracing::warn!(target: "autostart", %e, "Windows backend init failed, using NoOp");
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        match linux::LinuxAutostart::new() {
            Ok(l) => return Box::new(l),
            Err(e) => {
                tracing::warn!(target: "autostart", %e, "Linux backend init failed, using NoOp");
            }
        }
    }
    Box::new(noop::NoOpAutostart::new())
}

#[cfg(test)]
mod factory_tests {
    use super::*;

    #[test]
    fn new_for_current_platform_returns_a_manager() {
        let m = new_for_current_platform();
        // is_enabled may succeed or fail depending on platform/runner state;
        // we only assert the call doesn't panic and the trait object lives.
        let _ = m.is_enabled();
    }
}
