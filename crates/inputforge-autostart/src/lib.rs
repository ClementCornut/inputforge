//! Autostart manager for `InputForge`: writes OS-level launch-at-sign-in state
//! (HKCU registry on Windows, `~/.config/autostart/*.desktop` on Linux).
//!
//! See `docs/superpowers/specs/2026-05-10-f16-startup-preferences-design.md`.

mod error;

pub use error::AutostartError;
