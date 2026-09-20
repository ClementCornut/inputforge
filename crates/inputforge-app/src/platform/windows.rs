use anyhow::Result;

use inputforge_core::device::Sdl3Input;
use inputforge_core::output::mouse::MouseOutput;
use inputforge_core::output::{KeyboardOutput, VJoyOutput};

use super::PlatformBackends;

pub(super) fn create() -> Result<PlatformBackends> {
    Ok(PlatformBackends {
        input: Box::new(Sdl3Input::new()?),
        controller: Box::new(VJoyOutput::new()),
        keyboard: Box::new(KeyboardOutput::new()),
        mouse: Box::new(MouseOutput::new()),
    })
}
