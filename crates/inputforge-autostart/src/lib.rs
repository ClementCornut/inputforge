//! Autostart manager for `InputForge`: writes OS-level launch-at-sign-in state
//! (HKCU registry on Windows, `~/.config/autostart/*.desktop` on Linux).
//!
//! See `docs/superpowers/specs/2026-05-10-f16-startup-preferences-design.md`.

mod error;
mod noop;

pub use error::AutostartError;

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
