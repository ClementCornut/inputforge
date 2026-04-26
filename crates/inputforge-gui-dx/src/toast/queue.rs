//! Signal wrapper over `ToastState`. Constructed by the parent scope via
//! `use_signal(ToastState::default)`, then placed in context.
//!
//! Producers reach the queue with `use_context::<ToastQueue>().push(level, msg)`.
//! The viewport reads via `queue.visible(now)`.

use std::time::Instant;

use dioxus::prelude::*;

use crate::toast::state::{Toast, ToastLevel, ToastState, is_expired};

/// `Signal<ToastState>` wrapper. `Copy` (Signals are `Copy` in Dioxus 0.7),
/// so `ToastQueue` is freely passed by value into closures and contexts.
///
/// `state` is `pub` (rather than `pub(crate)`) so external example binaries
/// (Cargo `examples/`) and downstream crates can construct a queue. Producers
/// MUST initialize the inner Signal via `use_signal(ToastState::default)` from
/// inside a Dioxus runtime — `Signal::new()` outside a hook leaks per
/// `dioxus-signals/src/signal.rs:30-52`. Production wiring lives in `app_root`.
#[derive(Debug, Clone, Copy)]
pub struct ToastQueue {
    pub state: Signal<ToastState>,
}

impl ToastQueue {
    pub fn push(&self, level: ToastLevel, message: impl Into<String>) {
        // `Signal<T>` is `Copy`; rebinding to a `mut` local gives us the
        // `&mut self` that `WritableExt::write` requires without forcing the
        // method receiver to `&mut self` (Copy semantics — no aliasing risk).
        let mut state = self.state;
        state.write().push(level, message);
    }

    pub fn dismiss(&self, id: u64) {
        let mut state = self.state;
        state.write().dismiss(id);
    }

    pub fn pause(&self, id: u64) {
        let mut state = self.state;
        state.write().pause(id);
    }

    pub fn resume(&self, id: u64) {
        let mut state = self.state;
        state.write().resume(id);
    }

    /// Snapshot of non-expired toasts at `now`. Used by `ToastViewport` on
    /// each tick. Cloning is cheap (toasts are short and bounded by
    /// `TOAST_MAX_VISIBLE` plus a few in-flight dismissed entries fading out).
    #[expect(dead_code, reason = "called by ToastViewport in F4 Task 7")]
    pub(crate) fn visible(&self, now: Instant) -> Vec<Toast> {
        self.state
            .read()
            .toasts
            .iter()
            .filter(|t| !is_expired(t, now))
            .cloned()
            .collect()
    }
}
