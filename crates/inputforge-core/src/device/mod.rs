// Rust guideline compliant 2026-03-02

pub mod traits;

#[cfg(feature = "sdl3-input")]
pub mod sdl3;

#[cfg(feature = "win32-io")]
pub mod hidhide;

#[cfg(feature = "test-util")]
pub mod mock;
