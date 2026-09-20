//! Batch engine ownership; failures return before teardown so input can be released first.
use super::Output;
use crate::{
    error::{EngineError, Result},
    output::OutputSink,
    types::{HatDirection, VJoyAxis, VirtualDeviceConfig},
};

#[derive(Debug, Default)]
pub struct UinputSink {
    pub(super) output: Option<Output>,
    pub(super) failed_slot: Option<u8>,
}
impl UinputSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    fn active(&mut self) -> Result<&mut Output> {
        self.output
            .as_mut()
            .ok_or_else(|| EngineError::OutputFailed {
                reason: "start a virtual output session first".into(),
            })
    }
    fn write(&mut self, neutral: bool) -> Result<()> {
        let output = self.active()?;
        output
            .ensure_active()
            .map_err(|error| output_error(&error))?;
        if neutral {
            for held in &mut output.held {
                held.state.neutral();
            }
        }
        match output.flush_deferred(false) {
            Ok(()) => Ok(()),
            Err((slot, error)) => {
                self.failed_slot = Some(slot);
                Err(output_error(&error))
            }
        }
    }
}
impl OutputSink for UinputSink {
    fn capabilities(&self) -> crate::output::traits::ControllerCapabilities {
        crate::output::traits::ControllerCapabilities {
            configurable: true,
            min_buttons: 2,
            max_buttons: 53,
            max_hats: 4,
        }
    }
    fn start(&mut self, configs: &[VirtualDeviceConfig]) -> Result<()> {
        if self.output.is_some() {
            return Err(EngineError::OutputFailed {
                reason: "Stop before changing virtual devices".into(),
            });
        }
        let mut output = Output::new(configs.to_vec()).map_err(|error| output_error(&error))?;
        output.create().map_err(|error| output_error(&error))?;
        self.output = Some(output);
        self.failed_slot = None;
        Ok(())
    }
    fn neutralize(&mut self) -> Result<()> {
        if self.output.is_none() {
            return Ok(());
        }
        self.write(true)
    }
    fn stop(&mut self) -> Result<()> {
        let failed = self.failed_slot.take();
        self.output.take().map_or(Ok(()), |mut o| {
            o.release_all(failed).map_err(|error| output_error(&error))
        })
    }
    fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()> {
        self.active()?
            .set_axis(device, axis, value)
            .map_err(|error| output_error(&error))
    }
    fn set_button(&mut self, device: u8, button: u8, pressed: bool) -> Result<()> {
        self.active()?
            .set_button(device, button, pressed)
            .map_err(|error| output_error(&error))?;
        self.flush()
    }
    fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) -> Result<()> {
        self.active()?
            .set_hat(device, hat, direction)
            .map_err(|error| output_error(&error))?;
        self.flush()
    }
    fn flush(&mut self) -> Result<()> {
        if self.output.is_none() {
            return Ok(());
        }
        self.write(false)
    }
    fn list_devices(&self) -> Vec<VirtualDeviceConfig> {
        self.output
            .as_ref()
            .map_or_else(Vec::new, |o| o.configs().to_vec())
    }
}
fn output_error(error: &super::Error) -> EngineError {
    EngineError::OutputFailed {
        reason: format!("{error}; check /dev/uinput access externally, then Retry"),
    }
}
