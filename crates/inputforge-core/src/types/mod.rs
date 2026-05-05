// Rust guideline compliant 2026-03-02

pub mod address;
pub mod device;
pub mod input;
pub mod mapping;

pub use address::{InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};
pub use device::{
    AxisPolarity, DeviceBatteryState, DeviceConnectionState, DeviceDiagnostics, DeviceId,
    DeviceInfo, VirtualDeviceConfig,
};
pub use input::{AxisValue, HatDirection, InputEvent, InputValue};
pub use mapping::{KeyCombo, KeyModifier, MergeOp};
