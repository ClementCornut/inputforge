use super::PlatformBackends;
use anyhow::Result;
use inputforge_core::{
    device::evdev::EvdevInput,
    output::uinput::{Keyboard, Mouse, UinputSink},
};
#[expect(
    clippy::unnecessary_wraps,
    reason = "matches the fallible platform contract on Windows"
)]
pub(super) fn create() -> Result<PlatformBackends> {
    Ok(PlatformBackends {
        input: Box::new(EvdevInput::new()),
        controller: Box::new(UinputSink::new()),
        keyboard: Box::new(Keyboard::new()),
        mouse: Box::new(Mouse::new()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_backends_are_real_supported_and_inert() {
        let backends = create().unwrap();
        assert!(backends.input.supports_exclusive());
        assert!(backends.input.enumerate_devices().is_empty());
        assert!(backends.controller.list_devices().is_empty());
        assert!(backends.keyboard.supported());
        assert!(backends.mouse.supported());
    }
}
