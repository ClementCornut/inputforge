use super::{
    device::DeviceSpec,
    event_device::{EventDevice, EventError},
    key_codes,
};
use crate::{
    error::Result,
    output::{KeyboardSink, OutputKind},
    types::KeyCombo,
};

/// Inert Linux keyboard output; [`KeyboardSink::start`] acquires uinput.
#[derive(Debug)]
pub struct Keyboard {
    pub(in crate::output::uinput) device: EventDevice,
}

impl Keyboard {
    #[must_use]
    pub fn new() -> Self {
        Self {
            device: EventDevice::new(DeviceSpec::keyboard(key_codes::all_codes())),
        }
    }

    fn result(result: std::result::Result<(), EventError>) -> Result<()> {
        result.map_err(|error| error.into_engine(OutputKind::Keyboard))
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardSink for Keyboard {
    fn supported(&self) -> bool {
        true
    }

    fn start(&mut self) -> Result<()> {
        Self::result(self.device.start())
    }

    fn stop(&mut self) -> Result<()> {
        Self::result(self.device.stop())
    }

    fn key_down(&mut self, combo: &KeyCombo) -> Result<()> {
        Self::result(self.device.press(&key_codes::combo_codes(combo)))
    }

    fn key_up(&mut self, combo: &KeyCombo) -> Result<()> {
        Self::result(self.device.release(&key_codes::combo_codes(combo)))
    }
}
