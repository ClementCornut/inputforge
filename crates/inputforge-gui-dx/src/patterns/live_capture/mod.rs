//! Live-capture primitive — GUI-only modal state that subscribes to
//! `AppState.input_cache` and emits the next observed input event.
//!
//! Single-instance pattern: provided once via context in `app_root`.
//! Each consumer reads it via `use_context::<LiveCapture>()`. Starting
//! a new capture cancels any in-flight one — there is exactly one
//! capture at a time across the entire GUI.
//!
//! See the F8 spec for the full state-machine and Esc-priority rules.
//! Task 8 wires the Dioxus hook adapter; this module currently exposes
//! only the pure `LiveCaptureCore::step` function and its config types.

mod machine;
#[cfg(test)]
mod tests;

#[allow(unused_imports, reason = "consumed by Task 8 hook adapter")]
pub(crate) use machine::{AXIS_DEADBAND, CoreState, DEBOUNCE_MS, InputKind, LiveCaptureCore};

/// Filter governing which input kinds the primitive accepts. F9-F12
/// will use `AxesOnly` / `ButtonsOnly` to discriminate range-record vs.
/// button-bind flows. F8's `+ Add mapping` always uses `Any`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(
    dead_code,
    reason = "AxesOnly / ButtonsOnly are exercised by tests; lib consumers land in F9-F12"
)]
pub(crate) enum CaptureFilter {
    #[default]
    Any,
    AxesOnly,
    ButtonsOnly,
}
