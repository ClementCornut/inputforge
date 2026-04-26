//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod queue;
pub(crate) mod state;
// viewport, warnings_bridge are added in later tasks.

pub use queue::ToastQueue;
pub use state::{TOAST_DURATION, TOAST_MAX_VISIBLE, Toast, ToastLevel, ToastState, is_expired};
