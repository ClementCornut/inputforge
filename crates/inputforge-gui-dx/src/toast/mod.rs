//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod queue;
pub(crate) mod state;
pub(crate) mod viewport;
pub(crate) mod warnings_bridge;

pub use queue::ToastQueue;
pub use state::{TOAST_DURATION, TOAST_MAX_VISIBLE, Toast, ToastLevel, ToastState, is_expired};
pub use viewport::ToastViewport;
pub(crate) use warnings_bridge::install_warnings_bridge;
