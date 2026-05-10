//! Test double for [`AutostartManager`]. Records every `set_enabled` call,
//! lets tests seed `is_enabled()` results and queue one-shot failures.
//!
//! Cloning shares state via `Arc<Mutex<>>`, so tests can hold one clone for
//! inspection while the engine owns another.
//!
//! `AutostartError` is intentionally not `Clone` (it carries `io::Error`).
//! Tests that need to assert on a specific error variant must therefore
//! seed a *factory closure* (`Fn() -> AutostartError`) rather than a
//! pre-constructed error value: each call to `is_enabled()` or
//! `set_enabled()` materialises a fresh error of the exact variant the
//! test asked for.

use std::sync::{Arc, Mutex};

use crate::{AutostartError, AutostartManager};

/// Recorded call to [`MockAutostart::set_enabled`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEnabledCall {
    pub enabled: bool,
    pub args: Vec<String>,
}

/// Factory producing an [`AutostartError`] on demand. Each invocation must
/// return a fresh error of the exact variant the test wants to observe.
type ErrorFactory = Box<dyn Fn() -> AutostartError + Send>;

#[derive(Default)]
struct State {
    is_enabled_value: bool,
    is_enabled_factory: Option<ErrorFactory>,
    set_enabled_calls: Vec<SetEnabledCall>,
    next_set_enabled_factory: Option<ErrorFactory>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("is_enabled_value", &self.is_enabled_value)
            .field("is_enabled_factory", &self.is_enabled_factory.is_some())
            .field("set_enabled_calls", &self.set_enabled_calls)
            .field(
                "next_set_enabled_factory",
                &self.next_set_enabled_factory.is_some(),
            )
            .finish()
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

    /// Set the value returned by future `is_enabled()` calls. Has no effect
    /// when an error factory is also seeded; clear it via
    /// [`Self::clear_is_enabled_error`] if needed.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn set_is_enabled_value(&self, value: bool) {
        self.inner.lock().unwrap().is_enabled_value = value;
    }

    /// Seed an error factory invoked by every future `is_enabled()` call.
    /// The closure must return a fresh `AutostartError` on each invocation
    /// (so the test sees the exact variant it asked for, not a string-erased
    /// substitute).
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn set_is_enabled_error<F>(&self, factory: F)
    where
        F: Fn() -> AutostartError + Send + 'static,
    {
        self.inner.lock().unwrap().is_enabled_factory = Some(Box::new(factory));
    }

    /// Drop any previously-seeded `is_enabled` error factory.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn clear_is_enabled_error(&self) {
        self.inner.lock().unwrap().is_enabled_factory = None;
    }

    /// Queue a single failure for the next `set_enabled` call. The factory
    /// is consumed on the next call; subsequent calls succeed unless this
    /// is called again.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn fail_next_set_enabled<F>(&self, factory: F)
    where
        F: Fn() -> AutostartError + Send + 'static,
    {
        self.inner.lock().unwrap().next_set_enabled_factory = Some(Box::new(factory));
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
        if let Some(f) = &state.is_enabled_factory {
            return Err(f());
        }
        Ok(state.is_enabled_value)
    }

    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError> {
        let mut state = self.inner.lock().unwrap();
        if let Some(factory) = state.next_set_enabled_factory.take() {
            return Err(factory());
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
        m.fail_next_set_enabled(|| AutostartError::RegistryDenied);
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
    fn seeded_is_enabled_error_surfaces_exact_variant() {
        let m = MockAutostart::new();
        m.set_is_enabled_error(|| AutostartError::NotSupported);
        let err = m.is_enabled().unwrap_err();
        // The whole point of the factory: variant is preserved end-to-end.
        assert!(matches!(err, AutostartError::NotSupported));
    }

    #[test]
    fn set_is_enabled_value_returns_ok_when_no_error_seeded() {
        let m = MockAutostart::new();
        m.set_is_enabled_value(true);
        assert!(m.is_enabled().unwrap());
    }

    #[test]
    fn clear_is_enabled_error_restores_value_path() {
        let m = MockAutostart::new();
        m.set_is_enabled_value(true);
        m.set_is_enabled_error(|| AutostartError::NotSupported);
        let _err = m.is_enabled().unwrap_err();
        m.clear_is_enabled_error();
        assert!(m.is_enabled().unwrap());
    }
}
