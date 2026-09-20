//! Ordered runtime delivery. A sampled state is never an input edge.
use crate::types::{DeviceId, InputEvent};

#[derive(Debug, Clone)]
pub enum InputUpdate {
    Frame(Vec<InputEvent>),
    Snapshot {
        device: DeviceId,
        values: Vec<InputEvent>,
        recovered: bool,
    },
    /// Trustworthy native resting sample; provisional startup samples must not use this.
    AxisSample {
        device: DeviceId,
        index: u8,
        value: f64,
    },
    Reset {
        device: DeviceId,
    },
}
