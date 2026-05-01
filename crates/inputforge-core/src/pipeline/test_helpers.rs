// Rust guideline compliant 2026-03-03

use std::collections::HashMap;

use super::InputCache;
use crate::types::{AxisPolarity, DeviceId, HatDirection, InputAddress, InputId};

/// Shared mock for [`InputCache`] used across pipeline tests.
///
/// Axes default to [`AxisPolarity::Bipolar`]; tests that need unipolar
/// behavior insert into `axis_polarities` directly.
pub(super) struct MockCache {
    pub buttons: HashMap<InputAddress, bool>,
    pub axes: HashMap<InputAddress, f64>,
    pub axis_polarities: HashMap<InputAddress, AxisPolarity>,
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
            axis_polarities: HashMap::new(),
            hats: HashMap::new(),
        }
    }
}

impl InputCache for MockCache {
    fn get_button(&self, address: &InputAddress) -> bool {
        self.buttons.get(address).copied().unwrap_or(false)
    }

    fn get_axis(&self, address: &InputAddress) -> (f64, AxisPolarity) {
        let value = self.axes.get(address).copied().unwrap_or(0.0);
        let polarity = self
            .axis_polarities
            .get(address)
            .copied()
            .unwrap_or_default();
        (value, polarity)
    }

    fn get_hat(&self, address: &InputAddress) -> HatDirection {
        self.hats
            .get(address)
            .copied()
            .unwrap_or(HatDirection::Center)
    }
}
