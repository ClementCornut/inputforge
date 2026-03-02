// Rust guideline compliant 2026-03-02

use std::collections::HashMap;

use super::InputCache;
use crate::types::{HatDirection, InputAddress};

/// Shared mock for [`InputCache`] used across pipeline tests.
pub(super) struct MockCache {
    pub buttons: HashMap<InputAddress, bool>,
    pub axes: HashMap<InputAddress, f64>,
    pub hats: HashMap<InputAddress, HatDirection>,
}

impl MockCache {
    pub fn new() -> Self {
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
