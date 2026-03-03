// Rust guideline compliant 2026-03-03

use std::collections::HashMap;

use crate::types::InputAddress;

/// Unique identifier for a registered callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallbackId(u64);

/// An action to execute when a button is released.
pub enum ReleaseCallback {
    /// Pop the current temporary mode, returning to the previous mode.
    PopTemporaryMode,
    /// Execute an arbitrary closure once.
    Custom(Box<dyn FnOnce() + Send>),
}

impl std::fmt::Debug for ReleaseCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PopTemporaryMode => write!(f, "PopTemporaryMode"),
            Self::Custom(_) => write!(f, "Custom(...)"),
        }
    }
}

/// Registry that tracks callbacks to fire when specific buttons are released.
#[derive(Debug, Default)]
pub struct CallbackRegistry {
    next_id: u64,
    entries: HashMap<InputAddress, Vec<(CallbackId, ReleaseCallback)>>,
}

impl CallbackRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a callback to fire when the button at `input` is released.
    ///
    /// # Panics
    ///
    /// Panics if the internal ID counter overflows (after 2^64 registrations).
    pub fn register(&mut self, input: InputAddress, callback: ReleaseCallback) -> CallbackId {
        let id = CallbackId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("CallbackRegistry ID space exhausted");
        self.entries.entry(input).or_default().push((id, callback));
        id
    }

    /// Fire and remove all callbacks registered for the given input.
    #[must_use = "fired callbacks must be executed by the engine"]
    pub fn fire(&mut self, input: &InputAddress) -> Vec<ReleaseCallback> {
        self.entries
            .remove(input)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, cb)| cb)
            .collect()
    }

    /// Remove all registered callbacks.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Cancel a specific callback by its ID.
    ///
    /// Returns `true` if the callback was found and removed.
    pub fn cancel(&mut self, id: CallbackId) -> bool {
        let mut found = false;
        for callbacks in self.entries.values_mut() {
            if let Some(pos) = callbacks.iter().position(|(cb_id, _)| *cb_id == id) {
                callbacks.swap_remove(pos);
                found = true;
                break;
            }
        }
        if found {
            self.entries.retain(|_, v| !v.is_empty());
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, InputId};

    fn button_addr(index: u8) -> InputAddress {
        InputAddress {
            device: DeviceId("stick-1".to_owned()),
            input: InputId::Button { index },
        }
    }

    #[test]
    fn register_and_fire_returns_callback() {
        let mut registry = CallbackRegistry::new();
        let addr = button_addr(0);
        registry.register(addr.clone(), ReleaseCallback::PopTemporaryMode);

        let fired = registry.fire(&addr);
        assert_eq!(fired.len(), 1);
        assert!(matches!(fired[0], ReleaseCallback::PopTemporaryMode));
    }

    #[test]
    fn fire_removes_callbacks() {
        let mut registry = CallbackRegistry::new();
        let addr = button_addr(0);
        registry.register(addr.clone(), ReleaseCallback::PopTemporaryMode);

        let fired = registry.fire(&addr);
        assert_eq!(fired.len(), 1);

        // Firing again returns empty
        let fired_again = registry.fire(&addr);
        assert!(fired_again.is_empty());
    }

    #[test]
    fn multiple_callbacks_on_same_input() {
        let mut registry = CallbackRegistry::new();
        let addr = button_addr(0);
        registry.register(addr.clone(), ReleaseCallback::PopTemporaryMode);
        registry.register(addr.clone(), ReleaseCallback::PopTemporaryMode);

        let fired = registry.fire(&addr);
        assert_eq!(fired.len(), 2);
    }

    #[test]
    fn cancel_removes_specific_callback() {
        let mut registry = CallbackRegistry::new();
        let addr = button_addr(0);
        let id_a = registry.register(addr.clone(), ReleaseCallback::PopTemporaryMode);
        let _id_b = registry.register(addr.clone(), ReleaseCallback::PopTemporaryMode);

        let removed = registry.cancel(id_a);
        assert!(removed);

        // Only one callback should remain
        let fired = registry.fire(&addr);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn cancel_nonexistent_returns_false() {
        let mut registry = CallbackRegistry::new();
        let fake_id = CallbackId(999);
        assert!(!registry.cancel(fake_id));
    }

    #[test]
    fn fire_unregistered_input_returns_empty() {
        let mut registry = CallbackRegistry::new();
        let addr = button_addr(0);
        let fired = registry.fire(&addr);
        assert!(fired.is_empty());
    }

    #[test]
    fn callbacks_on_different_inputs_are_independent() {
        let mut registry = CallbackRegistry::new();
        let addr_a = button_addr(0);
        let addr_b = button_addr(1);
        registry.register(addr_a.clone(), ReleaseCallback::PopTemporaryMode);
        registry.register(addr_b.clone(), ReleaseCallback::PopTemporaryMode);

        let fired_a = registry.fire(&addr_a);
        assert_eq!(fired_a.len(), 1);

        // addr_b is unaffected
        let fired_b = registry.fire(&addr_b);
        assert_eq!(fired_b.len(), 1);
    }

    #[test]
    fn custom_callback_fires() {
        use std::sync::{Arc, Mutex};

        let mut registry = CallbackRegistry::new();
        let addr = button_addr(0);
        let called = Arc::new(Mutex::new(false));
        let called_clone = Arc::clone(&called);

        registry.register(
            addr.clone(),
            ReleaseCallback::Custom(Box::new(move || {
                *called_clone.lock().unwrap() = true;
            })),
        );

        let fired = registry.fire(&addr);
        assert_eq!(fired.len(), 1);
        if let ReleaseCallback::Custom(f) = fired.into_iter().next().unwrap() {
            f();
        }
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn debug_formatting() {
        let pop = ReleaseCallback::PopTemporaryMode;
        assert_eq!(format!("{pop:?}"), "PopTemporaryMode");

        let custom = ReleaseCallback::Custom(Box::new(|| {}));
        assert_eq!(format!("{custom:?}"), "Custom(...)");

        let registry = CallbackRegistry::new();
        let debug = format!("{registry:?}");
        assert!(debug.contains("CallbackRegistry"));
    }

    #[test]
    fn callback_ids_are_unique() {
        let mut registry = CallbackRegistry::new();
        let addr = button_addr(0);
        let id_a = registry.register(addr.clone(), ReleaseCallback::PopTemporaryMode);
        let id_b = registry.register(addr, ReleaseCallback::PopTemporaryMode);
        assert_ne!(id_a, id_b);
    }
}
