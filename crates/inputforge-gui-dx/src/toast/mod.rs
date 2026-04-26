//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod state;
// queue, viewport, warnings_bridge are added in later tasks.

pub use state::{TOAST_DURATION, TOAST_MAX_VISIBLE, Toast, ToastLevel, ToastState};
