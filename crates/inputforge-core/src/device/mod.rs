// Rust guideline compliant 2026-03-06

pub mod noop_hider;
pub mod traits;

#[cfg(feature = "sdl3-input")]
pub mod sdl3;

#[cfg(feature = "win32-io")]
pub mod hidhide;

#[cfg(feature = "test-util")]
pub mod mock;

pub use noop_hider::NoOpDeviceHider;
pub use traits::{DeviceHider, HotplugEvent, InputSource};

#[cfg(feature = "sdl3-input")]
pub use sdl3::Sdl3Input;

#[cfg(feature = "win32-io")]
pub use hidhide::HidHideManager;

#[cfg(feature = "test-util")]
pub use mock::{MockDeviceHider, MockInputSource};
