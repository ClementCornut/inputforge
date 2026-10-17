//! Prepared sampled input and output values.
use inputforge_core::{
    pipeline::OutputOwner,
    state::OutputActivityValue,
    types::{AxisPolarity, HatDirection, VJoyAxis},
};
/// Sampled input, output, and capture-session data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LiveSnapshot {
    pub capture_epoch: CaptureEpoch,
    pub input_readouts: std::collections::HashMap<
        inputforge_core::types::InputAddress,
        crate::readouts::InputReadout,
    >,
    pub readout: Option<crate::readouts::Readout>,
    pub device_inputs: Vec<DeviceInputValues>,
    pub output_values: Vec<VjoyOutputValues>,
    pub output_activity: Vec<OutputActivitySnapshot>,
}

/// One active output owned by a mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputActivitySnapshot {
    pub owner: OutputOwner,
    pub value: OutputActivityValue,
}

/// Sampled axes, buttons, and hats for one physical device.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeviceInputValues {
    pub axes: Vec<(f64, AxisPolarity)>,
    pub buttons: Vec<bool>,
    pub hats: Vec<HatDirection>,
}

/// Sampled axes, buttons, and hats for one virtual device.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VjoyOutputValues {
    pub axes: Vec<(VJoyAxis, f64)>,
    pub buttons: Vec<bool>,
    pub hats: Vec<HatDirection>,
}
/// Capture eligibility and generation from the sampled session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaptureEpoch {
    pub generation: u64,
    pub ready: bool,
    pub allowed: bool,
}
