// Rust guideline compliant 2026-03-03

use crate::action::MouseTarget;
use crate::error::Result;
use crate::types::{HatDirection, KeyCombo, PhysicalKey, VJoyAxis, VirtualDeviceConfig};

use super::traits::{KeyboardSink, MouseSink, OutputSink};

/// A recorded output call for test assertions.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputCall {
    CreateDevice(VirtualDeviceConfig),
    SetAxis {
        device: u8,
        axis: VJoyAxis,
        value: f64,
    },
    SetButton {
        device: u8,
        button: u8,
        pressed: bool,
    },
    SetHat {
        device: u8,
        hat: u8,
        direction: HatDirection,
    },
    ReleaseDevice(u8),
    Flush,
}

/// Mock output sink that records all calls for test assertions.
#[derive(Debug, Default)]
pub struct MockOutputSink {
    calls: Vec<OutputCall>,
}

impl MockOutputSink {
    /// Create a new empty `MockOutputSink`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all recorded calls.
    #[must_use]
    pub fn calls(&self) -> &[OutputCall] {
        &self.calls
    }

    /// Clear all recorded calls.
    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl OutputSink for MockOutputSink {
    fn create_device(&mut self, config: &VirtualDeviceConfig) -> Result<()> {
        self.calls.push(OutputCall::CreateDevice(config.clone()));
        Ok(())
    }

    fn set_axis(&mut self, device: u8, axis: VJoyAxis, value: f64) -> Result<()> {
        self.calls.push(OutputCall::SetAxis {
            device,
            axis,
            value,
        });
        Ok(())
    }

    fn set_button(&mut self, device: u8, button: u8, pressed: bool) -> Result<()> {
        self.calls.push(OutputCall::SetButton {
            device,
            button,
            pressed,
        });
        Ok(())
    }

    fn set_hat(&mut self, device: u8, hat: u8, direction: HatDirection) -> Result<()> {
        self.calls.push(OutputCall::SetHat {
            device,
            hat,
            direction,
        });
        Ok(())
    }

    fn release_device(&mut self, device: u8) -> Result<()> {
        self.calls.push(OutputCall::ReleaseDevice(device));
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.calls.push(OutputCall::Flush);
        Ok(())
    }
}

/// A recorded keyboard call for test assertions.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardCall {
    KeyDown(KeyCombo),
    KeyUp(KeyCombo),
    PulseKey(KeyCombo),
}

/// Mock keyboard sink that records all calls for test assertions.
#[derive(Debug, Default)]
pub struct MockKeyboardSink {
    calls: Vec<KeyboardCall>,
}

impl MockKeyboardSink {
    /// Create a new empty `MockKeyboardSink`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all recorded calls.
    #[must_use]
    pub fn calls(&self) -> &[KeyboardCall] {
        &self.calls
    }

    /// Clear all recorded calls.
    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl KeyboardSink for MockKeyboardSink {
    fn key_down(&mut self, combo: &KeyCombo) -> Result<()> {
        self.calls.push(KeyboardCall::KeyDown(combo.clone()));
        Ok(())
    }

    fn key_up(&mut self, combo: &KeyCombo) -> Result<()> {
        self.calls.push(KeyboardCall::KeyUp(combo.clone()));
        Ok(())
    }

    fn pulse_key(&mut self, combo: &KeyCombo) -> Result<()> {
        self.calls.push(KeyboardCall::PulseKey(combo.clone()));
        Ok(())
    }
}

/// A recorded mouse call for test assertions.
#[derive(Debug, Clone, PartialEq)]
pub enum MouseCall {
    ButtonDown(MouseTarget),
    ButtonUp(MouseTarget),
    PulseButton(MouseTarget),
    Wheel(MouseTarget),
}

/// Mock mouse sink that records all calls for test assertions.
#[derive(Debug, Default)]
pub struct MockMouseSink {
    calls: Vec<MouseCall>,
}

impl MockMouseSink {
    /// Create a new empty `MockMouseSink`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all recorded calls.
    #[must_use]
    pub fn calls(&self) -> &[MouseCall] {
        &self.calls
    }

    /// Clear all recorded calls.
    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl MouseSink for MockMouseSink {
    fn button_down(&mut self, target: MouseTarget) -> Result<()> {
        self.calls.push(MouseCall::ButtonDown(target));
        Ok(())
    }

    fn button_up(&mut self, target: MouseTarget) -> Result<()> {
        self.calls.push(MouseCall::ButtonUp(target));
        Ok(())
    }

    fn pulse_button(&mut self, target: MouseTarget) -> Result<()> {
        self.calls.push(MouseCall::PulseButton(target));
        Ok(())
    }

    fn wheel(&mut self, target: MouseTarget) -> Result<()> {
        self.calls.push(MouseCall::Wheel(target));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_records_create_device() {
        let mut mock = MockOutputSink::new();
        let config = VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X, VJoyAxis::Y],
            button_count: 8,
            hat_count: 1,
        };
        mock.create_device(&config).unwrap();
        assert_eq!(mock.calls(), &[OutputCall::CreateDevice(config)]);
    }

    #[test]
    fn mock_records_set_axis() {
        let mut mock = MockOutputSink::new();
        mock.set_axis(1, VJoyAxis::X, 0.5).unwrap();
        assert_eq!(
            mock.calls(),
            &[OutputCall::SetAxis {
                device: 1,
                axis: VJoyAxis::X,
                value: 0.5,
            }]
        );
    }

    #[test]
    fn mock_records_set_button() {
        let mut mock = MockOutputSink::new();
        mock.set_button(1, 3, true).unwrap();
        assert_eq!(
            mock.calls(),
            &[OutputCall::SetButton {
                device: 1,
                button: 3,
                pressed: true,
            }]
        );
    }

    #[test]
    fn mock_records_set_hat() {
        let mut mock = MockOutputSink::new();
        mock.set_hat(1, 1, HatDirection::NE).unwrap();
        assert_eq!(
            mock.calls(),
            &[OutputCall::SetHat {
                device: 1,
                hat: 1,
                direction: HatDirection::NE,
            }]
        );
    }

    #[test]
    fn mock_records_release_device() {
        let mut mock = MockOutputSink::new();
        mock.release_device(2).unwrap();
        assert_eq!(mock.calls(), &[OutputCall::ReleaseDevice(2)]);
    }

    #[test]
    fn mock_records_flush() {
        let mut mock = MockOutputSink::new();
        mock.flush().unwrap();
        assert_eq!(mock.calls(), &[OutputCall::Flush]);
    }

    #[test]
    fn mock_clear_removes_all_calls() {
        let mut mock = MockOutputSink::new();
        mock.set_axis(1, VJoyAxis::X, 0.0).unwrap();
        mock.set_button(1, 1, true).unwrap();
        assert_eq!(mock.calls().len(), 2);
        mock.clear();
        assert!(mock.calls().is_empty());
    }

    // --- MockKeyboardSink ---

    #[test]
    fn mock_keyboard_records_pulse_key() {
        use crate::types::KeyModifier;

        let mut mock = MockKeyboardSink::new();
        let combo = KeyCombo {
            key: PhysicalKey::Space,
            modifiers: vec![KeyModifier::Ctrl],
        };
        mock.pulse_key(&combo).unwrap();
        assert_eq!(mock.calls(), &[KeyboardCall::PulseKey(combo)]);
    }

    #[test]
    fn mock_keyboard_records_down_up_and_pulse() {
        let mut mock = MockKeyboardSink::new();
        let combo = KeyCombo {
            key: PhysicalKey::KeyA,
            modifiers: vec![],
        };

        mock.key_down(&combo).unwrap();
        mock.key_up(&combo).unwrap();
        mock.pulse_key(&combo).unwrap();

        assert_eq!(
            mock.calls(),
            &[
                KeyboardCall::KeyDown(combo.clone()),
                KeyboardCall::KeyUp(combo.clone()),
                KeyboardCall::PulseKey(combo),
            ]
        );
    }

    #[test]
    fn mock_mouse_records_button_and_wheel_calls() {
        let mut mock = MockMouseSink::new();

        mock.button_down(MouseTarget::LeftButton).unwrap();
        mock.button_up(MouseTarget::LeftButton).unwrap();
        mock.pulse_button(MouseTarget::RightButton).unwrap();
        mock.wheel(MouseTarget::WheelUp).unwrap();

        assert_eq!(
            mock.calls(),
            &[
                MouseCall::ButtonDown(MouseTarget::LeftButton),
                MouseCall::ButtonUp(MouseTarget::LeftButton),
                MouseCall::PulseButton(MouseTarget::RightButton),
                MouseCall::Wheel(MouseTarget::WheelUp),
            ]
        );
    }

    #[test]
    fn mock_keyboard_clear_removes_calls() {
        let mut mock = MockKeyboardSink::new();
        let combo = KeyCombo {
            key: PhysicalKey::KeyA,
            modifiers: vec![],
        };
        mock.key_down(&combo).unwrap();
        assert_eq!(mock.calls().len(), 1);
        mock.clear();
        assert!(mock.calls().is_empty());
    }
}
