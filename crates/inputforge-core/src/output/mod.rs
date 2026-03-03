// Rust guideline compliant 2026-03-03

pub mod traits;

#[cfg(feature = "vjoy-output")]
pub mod vjoy_output;

#[cfg(feature = "win32-io")]
pub mod keyboard;

#[cfg(feature = "test-util")]
pub mod mock;

pub use traits::{KeyboardSink, OutputSink, VirtualDeviceConfig};

#[cfg(feature = "vjoy-output")]
pub use vjoy_output::VJoyOutput;

#[cfg(feature = "win32-io")]
pub use keyboard::KeyboardOutput;

#[cfg(feature = "test-util")]
pub use mock::{MockOutputSink, OutputCall};
