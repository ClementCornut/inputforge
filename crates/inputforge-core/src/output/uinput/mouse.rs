use super::{
    device::DeviceSpec,
    event_device::{EventDevice, EventError},
};
use crate::{
    action::MouseTarget,
    error::{EngineError, Result},
    output::{MouseSink, OutputKind},
};
use evdev::{KeyCode, RelativeAxisCode};

/// Inert Linux mouse output; [`MouseSink::start`] acquires uinput.
#[derive(Debug)]
pub struct Mouse {
    pub(in crate::output::uinput) device: EventDevice,
}

impl Mouse {
    #[must_use]
    pub fn new() -> Self {
        Self {
            device: EventDevice::new(DeviceSpec::mouse()),
        }
    }

    fn result(result: std::result::Result<(), EventError>) -> Result<()> {
        result.map_err(|error| error.into_engine(OutputKind::Mouse))
    }
}

impl Default for Mouse {
    fn default() -> Self {
        Self::new()
    }
}

impl MouseSink for Mouse {
    fn supported(&self) -> bool {
        true
    }

    fn start(&mut self) -> Result<()> {
        Self::result(self.device.start())
    }

    fn stop(&mut self) -> Result<()> {
        Self::result(self.device.stop())
    }

    fn button_down(&mut self, target: MouseTarget) -> Result<()> {
        let code = button_code(target)?;
        Self::result(self.device.press(&[code]))
    }

    fn button_up(&mut self, target: MouseTarget) -> Result<()> {
        let code = button_code(target)?;
        Self::result(self.device.release(&[code]))
    }

    fn wheel(&mut self, target: MouseTarget) -> Result<()> {
        let value = match target {
            MouseTarget::WheelUp => 1,
            MouseTarget::WheelDown => -1,
            _ => return Err(invalid("wheel output requires a wheel target")),
        };
        Self::result(self.device.relative(RelativeAxisCode::REL_WHEEL.0, value))
    }
}

fn button_code(target: MouseTarget) -> Result<u16> {
    match target {
        MouseTarget::LeftButton => Ok(KeyCode::BTN_LEFT.0),
        MouseTarget::RightButton => Ok(KeyCode::BTN_RIGHT.0),
        MouseTarget::MiddleButton => Ok(KeyCode::BTN_MIDDLE.0),
        MouseTarget::BackButton => Ok(KeyCode::BTN_SIDE.0),
        MouseTarget::ForwardButton => Ok(KeyCode::BTN_EXTRA.0),
        MouseTarget::WheelUp | MouseTarget::WheelDown => {
            Err(invalid("button output requires a button target"))
        }
    }
}

fn invalid(reason: &str) -> EngineError {
    EngineError::InvalidConfig {
        reason: reason.to_owned(),
    }
}
