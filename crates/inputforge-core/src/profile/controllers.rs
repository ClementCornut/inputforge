//! Controller configuration shared by native input/output adapters.
use crate::{
    error::{EngineError, Result},
    types::{AxisPolarity, DeviceId, InputId, VirtualDeviceConfig},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerConfig {
    pub version: u8,
    #[serde(skip)]
    pub legacy_axis_settings: bool,
    pub selected: Vec<DeviceId>,
    pub bindings: Vec<DeviceBinding>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            version: 1,
            legacy_axis_settings: false,
            selected: Vec::new(),
            bindings: Vec::new(),
            virtual_devices: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceBinding {
    pub device: DeviceId,
    /// Last observed native layout, excluding retained logical tombstones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_layout: Option<ControlLayout>,
    #[serde(default)]
    pub unavailable: Vec<InputId>,
    pub axes: Vec<AxisBinding>,
    pub buttons: Vec<u16>,
    /// Native hat identifiers supplied by the adapter.
    pub hats: Vec<u8>,
}

/// Native control identities observed together in a positional device layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlLayout {
    pub axes: Vec<u16>,
    pub buttons: Vec<u16>,
    pub hats: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxisBinding {
    pub code: u16,
    pub minimum: i32,
    pub maximum: i32,
    #[serde(default)]
    pub polarity: AxisPolarity,
}

pub(crate) fn invalid(reason: impl Into<String>) -> EngineError {
    EngineError::InvalidConfig {
        reason: reason.into(),
    }
}

impl ControllerConfig {
    /// Validate the persisted description without opening hardware.
    /// # Errors
    /// Rejects unsupported versions, duplicate identities and malformed controls.
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err(invalid("unsupported controller binding version"));
        }
        for (i, id) in self.selected.iter().enumerate() {
            if id.0.is_empty() || self.selected[..i].contains(id) {
                return Err(invalid(format!(
                    "controller selection {} is empty or duplicated",
                    id.0
                )));
            }
        }
        for (i, binding) in self.bindings.iter().enumerate() {
            binding.validate()?;
            if self.bindings[..i]
                .iter()
                .any(|b| b.device == binding.device)
            {
                return Err(invalid("duplicate controller device binding"));
            }
        }
        for (i, config) in self.virtual_devices.iter().enumerate() {
            config.validate()?;
            if config.hat_count > 4
                || self.virtual_devices[..i]
                    .iter()
                    .any(|c| c.device_id == config.device_id)
                || config
                    .axes
                    .iter()
                    .enumerate()
                    .any(|(i, a)| config.axes[..i].contains(a))
            {
                return Err(invalid(format!(
                    "invalid controller virtual slot {}",
                    config.device_id
                )));
            }
        }
        Ok(())
    }
}

impl DeviceBinding {
    /// # Errors
    /// Rejects ambiguous, overflowing or invalid native tables.
    pub fn validate(&self) -> Result<()> {
        if self.device.0.is_empty()
            || self.axes.len() > 255
            || self.buttons.len() > 255
            || self.hats.len() > 4
            || self
                .observed_layout
                .as_ref()
                .is_some_and(|layout| !layout.valid_for(self))
        {
            return Err(invalid(format!(
                "invalid controller controls for {}",
                self.device.0
            )));
        }
        for (i, axis) in self.axes.iter().enumerate() {
            if axis.minimum >= axis.maximum || self.axes[..i].iter().any(|a| a.code == axis.code) {
                return Err(invalid(format!(
                    "invalid or duplicate axis {} for {}",
                    axis.code, self.device.0
                )));
            }
        }
        if self
            .buttons
            .iter()
            .enumerate()
            .any(|(i, c)| self.buttons[..i].contains(c))
            || self
                .hats
                .iter()
                .enumerate()
                .any(|(i, h)| *h > 3 || self.hats[..i].contains(h))
        {
            return Err(invalid(format!(
                "invalid or duplicate buttons/hats for {}",
                self.device.0
            )));
        }
        Ok(())
    }

    #[must_use]
    pub fn resolves(&self, input: &InputId) -> bool {
        if self.unavailable.contains(input) {
            return false;
        }
        match input {
            InputId::Axis { index } => usize::from(*index) < self.axes.len(),
            InputId::Button { index } => usize::from(*index) < self.buttons.len(),
            InputId::Hat { index } => usize::from(*index) < self.hats.len(),
        }
    }
}
