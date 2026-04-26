//! Toast queue: pure-data state + Signal wrapper + viewport component +
//! production warnings bridge.

pub(crate) mod queue;
pub(crate) mod state;
pub(crate) mod viewport;
// warnings_bridge is added in Task 16.

pub use queue::ToastQueue;
pub use state::{TOAST_DURATION, TOAST_MAX_VISIBLE, Toast, ToastLevel, ToastState, is_expired};
#[expect(
    unused_imports,
    reason = "consumed by gallery_root in F4 Task 9 and app_root in F4 Task 16"
)]
pub(crate) use viewport::ToastViewport;
