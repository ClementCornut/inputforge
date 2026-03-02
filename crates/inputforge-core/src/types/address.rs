// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use super::device::DeviceId;

/// Fully qualified address of a physical input (device + input).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InputAddress {
    pub device: DeviceId,
    pub input: InputId,
}

/// Identifies a specific input on a device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputId {
    Axis { index: u8 },
    Button { index: u8 },
    Hat { index: u8 },
}

/// Fully qualified address of a virtual output (vJoy device + output).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputAddress {
    pub device: u8,
    pub output: OutputId,
}

/// Identifies a specific output on a vJoy device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputId {
    Axis { id: VJoyAxis },
    Button { id: u8 },
    Hat { id: u8 },
}

/// vJoy axis identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VJoyAxis {
    X,
    Y,
    Z,
    Rx,
    Ry,
    Rz,
    Slider0,
    Slider1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_id_axis_serde_roundtrip() {
        let id = InputId::Axis { index: 3 };
        let json = serde_json::to_string(&id).unwrap();
        let back: InputId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn input_id_button_serde_roundtrip() {
        let id = InputId::Button { index: 7 };
        let json = serde_json::to_string(&id).unwrap();
        let back: InputId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn input_id_hat_serde_roundtrip() {
        let id = InputId::Hat { index: 0 };
        let json = serde_json::to_string(&id).unwrap();
        let back: InputId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn output_id_axis_serde_roundtrip() {
        let id = OutputId::Axis { id: VJoyAxis::X };
        let json = serde_json::to_string(&id).unwrap();
        let back: OutputId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn input_address_serde_roundtrip() {
        let addr = InputAddress {
            device: DeviceId("guid-001".to_owned()),
            input: InputId::Axis { index: 2 },
        };
        let json = serde_json::to_string(&addr).unwrap();
        let back: InputAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, back);
    }

    #[test]
    fn vjoy_axis_all_variants() {
        let axes = [
            VJoyAxis::X,
            VJoyAxis::Y,
            VJoyAxis::Z,
            VJoyAxis::Rx,
            VJoyAxis::Ry,
            VJoyAxis::Rz,
            VJoyAxis::Slider0,
            VJoyAxis::Slider1,
        ];
        assert_eq!(axes.len(), 8);
    }
}
