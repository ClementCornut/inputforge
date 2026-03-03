// Rust guideline compliant 2026-03-03

use crate::error::Result;
use crate::types::{HatDirection, VJoyAxis, VirtualDeviceConfig};

use super::traits::OutputSink;

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
}
