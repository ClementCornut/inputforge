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
        assert_eq!(backends.input.enumerate_devices().len(), 0);
        assert_eq!(backends.controller.list_devices().len(), 0);
        assert!(backends.keyboard.supported());
        assert!(backends.mouse.supported());
    }
}
