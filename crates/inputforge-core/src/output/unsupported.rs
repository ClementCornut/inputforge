//! Production sinks for platforms without injection support.
use crate::{
    action::MouseTarget,
    error::{EngineError, Result},
    output::{KeyboardSink, MouseSink},
    types::KeyCombo,
};
#[derive(Debug, Default)]
pub struct Unsupported;
fn unsupported() -> Result<()> {
    Err(EngineError::OutputFailed {
        reason: "keyboard/mouse injection is unavailable on this platform".into(),
    })
}
impl KeyboardSink for Unsupported {
    fn supported(&self) -> bool {
        false
    }
    fn key_down(&mut self, _: &KeyCombo) -> Result<()> {
        unsupported()
    }
    fn key_up(&mut self, _: &KeyCombo) -> Result<()> {
        unsupported()
    }
}
impl MouseSink for Unsupported {
    fn supported(&self) -> bool {
        false
    }
    fn button_down(&mut self, _: MouseTarget) -> Result<()> {
        unsupported()
    }
    fn button_up(&mut self, _: MouseTarget) -> Result<()> {
        unsupported()
    }
    fn wheel(&mut self, _: MouseTarget) -> Result<()> {
        unsupported()
    }
}

#[cfg(test)]
mod tests {
    use super::Unsupported;
    use crate::{
        action::MouseTarget,
        output::{KeyboardSink, MouseSink},
        types::{KeyCombo, PhysicalKey},
    };
    #[test]
    fn production_unsupported_sinks_never_acknowledge_injection() {
        let mut sink = Unsupported;
        assert!(!KeyboardSink::supported(&sink));
        assert!(!MouseSink::supported(&sink));
        let key = KeyCombo {
            key: PhysicalKey::Space,
            modifiers: vec![],
        };
        assert!(sink.key_down(&key).is_err());
        assert!(sink.key_up(&key).is_err());
        assert!(sink.pulse_key(&key).is_err());
        assert!(sink.button_down(MouseTarget::LeftButton).is_err());
        assert!(sink.button_up(MouseTarget::LeftButton).is_err());
        assert!(sink.pulse_button(MouseTarget::LeftButton).is_err());
        assert!(sink.wheel(MouseTarget::WheelUp).is_err());
    }
}
