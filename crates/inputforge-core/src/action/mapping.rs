// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

use crate::types::InputAddress;

use super::Action;

/// A mapping from a physical input to a sequence of processing actions.
///
/// Placed in this module (not `types/mapping.rs`) because it references [`Action`],
/// which depends on `processing/`. Placing it there would create a circular dependency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mapping {
    pub input: InputAddress,
    pub mode: String,
    pub actions: Vec<Action>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::DeadzoneConfig;
    use crate::types::{DeviceId, InputId, OutputAddress, OutputId, VJoyAxis};

    fn test_input_address() -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

    fn test_output_address() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    #[test]
    fn mapping_serde_roundtrip() {
        let mapping = Mapping {
            input: test_input_address(),
            mode: "default".to_owned(),
            actions: vec![
                Action::Deadzone {
                    config: DeadzoneConfig::default(),
                },
                Action::Invert,
                Action::MapToVJoy {
                    output: test_output_address(),
                },
            ],
        };
        let json = serde_json::to_string(&mapping).unwrap();
        let back: Mapping = serde_json::from_str(&json).unwrap();
        assert_eq!(mapping, back);
    }
}
