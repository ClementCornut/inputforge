// Rust guideline compliant 2026-03-03

mod failure;
pub mod traits;

#[cfg(all(target_os = "linux", feature = "uinput-output"))]
pub mod uinput;

#[cfg(all(target_os = "windows", feature = "vjoy-output"))]
pub mod vjoy_output;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub mod keyboard;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub mod mouse;

#[cfg(any(test, feature = "test-util"))]
pub mod mock;

pub use failure::{OutputFailure, OutputKind, OutputPhase};
pub use traits::{KeyboardSink, MouseSink, OutputSink, VirtualDeviceConfig};

#[cfg(all(target_os = "windows", feature = "vjoy-output"))]
pub use vjoy_output::VJoyOutput;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub use keyboard::KeyboardOutput;

#[cfg(any(test, feature = "test-util"))]
pub use mock::{
    KeyboardCall, MockKeyboardSink, MockMouseSink, MockOutputSink, MouseCall, OutputCall,
};

pub mod unsupported;
#[cfg(all(target_os = "windows", feature = "vjoy-output"))]
mod vjoy_session;
