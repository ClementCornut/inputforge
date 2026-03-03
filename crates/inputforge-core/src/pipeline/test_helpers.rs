// Rust guideline compliant 2026-03-03

use std::collections::HashMap;

use super::InputCache;
use crate::types::{DeviceId, HatDirection, InputAddress, InputId};

/// Shared mock for [`InputCache`] used across pipeline tests.
pub(super) struct MockCache {
    pub buttons: HashMap<InputAddress, bool>,
    pub axes: HashMap<InputAddress, f64>,
    pub hats: HashMap<InputAddress, HatDirection>,
}

/// Shared button input address for pipeline tests.
pub(super) fn button_input_address() -> InputAddress {
    InputAddress {
        device: DeviceId("stick-1".to_owned()),
        input: InputId::Button { index: 0 },
    }
}

/// Shared axis input address for pipeline tests.
pub(super) fn axis_input_address() -> InputAddress {
    InputAddress {
        device: DeviceId("stick-1".to_owned()),
        input: InputId::Axis { index: 0 },
    }
}

impl MockCache {
    pub(super) fn new() -> Self {
        Self {
            buttons: HashMap::new(),
            axes: HashMap::new(),
            hats: HashMap::new(),
        }
    }
}

impl InputCache for MockCache {
    fn get_button(&self, address: &InputAddress) -> bool {
        self.buttons.get(address).copied().unwrap_or(false)
    }

    fn get_axis(&self, address: &InputAddress) -> f64 {
        self.axes.get(address).copied().unwrap_or(0.0)
    }

    fn get_hat(&self, address: &InputAddress) -> HatDirection {
        self.hats
            .get(address)
            .copied()
            .unwrap_or(HatDirection::Center)
    }
}
