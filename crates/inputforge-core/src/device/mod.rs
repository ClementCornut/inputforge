// Rust guideline compliant 2026-03-06

#[cfg(all(target_os = "linux", feature = "evdev-input"))]
pub mod evdev;

#[cfg(all(
    target_os = "linux",
    any(feature = "evdev-input", feature = "uinput-output")
))]
pub(crate) mod linux_bitmap;

pub mod noop_hider;
pub mod traits;

#[cfg(feature = "sdl3-input")]
pub mod sdl3;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub mod hidhide;

#[cfg(any(test, feature = "test-util"))]
pub mod mock;

pub use noop_hider::NoOpDeviceHider;
pub use traits::{DeviceHider, HotplugEvent, InputSource};

#[cfg(feature = "sdl3-input")]
pub use sdl3::Sdl3Input;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub use hidhide::HidHideManager;

#[cfg(any(test, feature = "test-util"))]
pub use mock::{MockDeviceHider, MockInputSource};

mod update;
pub use update::InputUpdate;
