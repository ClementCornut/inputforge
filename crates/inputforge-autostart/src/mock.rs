//! Test double for [`AutostartManager`]. Records every `set_enabled` call,
//! lets tests seed `is_enabled()` results and queue one-shot failures.
//!
//! Cloning shares state via `Arc<Mutex<>>`, so tests can hold one clone for
//! inspection while the engine owns another.

use std::sync::{Arc, Mutex};

use crate::{AutostartError, AutostartManager};

/// Recorded call to [`MockAutostart::set_enabled`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEnabledCall {
    pub enabled: bool,
    pub args: Vec<String>,
}

#[derive(Debug)]
struct State {
    is_enabled: Result<bool, AutostartError>,
    set_enabled_calls: Vec<SetEnabledCall>,
    next_set_enabled_failure: Option<AutostartError>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_enabled: Ok(false),
            set_enabled_calls: Vec::new(),
            next_set_enabled_failure: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MockAutostart {
    inner: Arc<Mutex<State>>,
}

impl MockAutostart {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the value returned by future `is_enabled()` calls. The argument is
    /// cloned each call; for the error path, store an `AutostartError`
    /// representative the test wants observed (e.g., `NotSupported`).
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "by-value matches the future call sites in inputforge-core engine tests; \
                  cloning is cheap (Result<bool, AutostartError>) and the API stays ergonomic"
    )]
    pub fn set_is_enabled_result(&self, result: Result<bool, AutostartError>) {
        let mut state = self.inner.lock().unwrap();
        state.is_enabled = match result {
            Ok(v) => Ok(v),
            Err(_) => Err(AutostartError::Backend("seeded mock error".to_owned())),
        };
    }

    /// Queue a single failure for the next `set_enabled` call. Subsequent
    /// calls succeed unless this is called again.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn fail_next_set_enabled(&self, err: AutostartError) {
        self.inner.lock().unwrap().next_set_enabled_failure = Some(err);
    }

    /// Snapshot of recorded `set_enabled` calls, in dispatch order.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn calls(&self) -> Vec<SetEnabledCall> {
        self.inner.lock().unwrap().set_enabled_calls.clone()
    }
}

impl AutostartManager for MockAutostart {
    fn is_enabled(&self) -> Result<bool, AutostartError> {
        let state = self.inner.lock().unwrap();
        match &state.is_enabled {
            Ok(v) => Ok(*v),
            Err(_) => Err(AutostartError::Backend("seeded mock error".to_owned())),
        }
    }

    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError> {
        let mut state = self.inner.lock().unwrap();
        if let Some(err) = state.next_set_enabled_failure.take() {
            return Err(err);
        }
        state.set_enabled_calls.push(SetEnabledCall {
            enabled,
            args: args.iter().map(|&s| s.to_owned()).collect(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_enabled_returns_false() {
        let m = MockAutostart::new();
        assert!(!m.is_enabled().unwrap());
    }

    #[test]
    fn set_enabled_records_calls_in_order() {
        let mut m = MockAutostart::new();
        m.set_enabled(true, &["--start-minimized"]).unwrap();
        m.set_enabled(false, &[]).unwrap();
        let calls = m.calls();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].enabled);
        assert_eq!(calls[0].args, vec!["--start-minimized".to_owned()]);
        assert!(!calls[1].enabled);
        assert!(calls[1].args.is_empty());
    }

    #[test]
    fn fail_next_set_enabled_consumes_one_call_then_succeeds() {
        let mut m = MockAutostart::new();
        m.fail_next_set_enabled(AutostartError::RegistryDenied);
        let err = m.set_enabled(true, &[]).unwrap_err();
        assert!(matches!(err, AutostartError::RegistryDenied));
        // Next call must now succeed.
        m.set_enabled(true, &[]).unwrap();
        assert_eq!(m.calls().len(), 1, "failed call must not be recorded");
    }

    #[test]
    fn clone_shares_state_with_original() {
        let mut a = MockAutostart::new();
        let b = a.clone();
        a.set_enabled(true, &[]).unwrap();
        assert_eq!(b.calls().len(), 1, "clone must observe parent's calls");
    }

    #[test]
    fn seeded_is_enabled_error_surfaces_through_trait() {
        let m = MockAutostart::new();
        m.set_is_enabled_result(Err(AutostartError::NotSupported));
        let err = m.is_enabled().unwrap_err();
        assert!(matches!(err, AutostartError::Backend(_)));
    }
}
