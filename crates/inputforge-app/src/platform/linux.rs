use super::PlatformBackends;
use anyhow::Result;
use inputforge_core::{
    device::evdev::EvdevInput,
    output::{uinput::UinputSink, unsupported::Unsupported},
};
#[expect(
    clippy::unnecessary_wraps,
    reason = "matches the fallible platform contract on Windows"
)]
pub(super) fn create() -> Result<PlatformBackends> {
    Ok(PlatformBackends {
        input: Box::new(EvdevInput::new()),
        controller: Box::new(UinputSink::new()),
        keyboard: Box::new(Unsupported),
        mouse: Box::new(Unsupported),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_backends_are_real_and_inert_until_polled() {
        let backends = create().unwrap();
        assert!(backends.input.supports_exclusive());
        assert!(backends.input.enumerate_devices().is_empty());
        assert!(backends.controller.list_devices().is_empty());
    }
}
