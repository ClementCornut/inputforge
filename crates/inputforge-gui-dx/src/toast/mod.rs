//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod queue;
pub(crate) mod state;
pub(crate) mod viewport;
// warnings_bridge is added in Task 16.

pub use queue::ToastQueue;
pub use state::{TOAST_DURATION, TOAST_MAX_VISIBLE, Toast, ToastLevel, ToastState, is_expired};
pub use viewport::ToastViewport;
