//! Fallback impl used when no platform backend is available (e.g., when
//! `std::env::current_exe()` fails). Reports disabled and rejects writes.

use crate::{AutostartError, AutostartManager};

#[derive(Debug, Default)]
#[allow(dead_code, reason = "used by the factory in Task 1.8; not yet wired")]
pub(crate) struct NoOpAutostart;

impl NoOpAutostart {
    #[allow(dead_code, reason = "called by the factory in Task 1.8; not yet wired")]
    pub(crate) fn new() -> Self {
        Self
    }
}

impl AutostartManager for NoOpAutostart {
    fn is_enabled(&self) -> Result<bool, AutostartError> {
        Ok(false)
    }

    fn set_enabled(&mut self, _enabled: bool, _args: &[&str]) -> Result<(), AutostartError> {
        Err(AutostartError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_enabled_reports_false() {
        let m = NoOpAutostart::new();
        assert!(!m.is_enabled().unwrap());
    }

    #[test]
    fn set_enabled_true_returns_not_supported() {
        let mut m = NoOpAutostart::new();
        let err = m.set_enabled(true, &["--start-minimized"]).unwrap_err();
        assert!(matches!(err, AutostartError::NotSupported));
    }

    #[test]
    fn set_enabled_false_returns_not_supported() {
        let mut m = NoOpAutostart::new();
        let err = m.set_enabled(false, &[]).unwrap_err();
        assert!(matches!(err, AutostartError::NotSupported));
    }
}
