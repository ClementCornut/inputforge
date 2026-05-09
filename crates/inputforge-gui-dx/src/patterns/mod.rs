//! Reusable composed-component patterns. F4 ships only `DirtyConfirmDialog`;
//! later features may add `SaveBeforeLeave`, `ConfirmDestructive`, etc.

pub mod destructive_confirm;
pub mod dirty_confirm;
pub mod live_capture;

pub use destructive_confirm::DestructiveConfirmDialog;
pub use dirty_confirm::DirtyConfirmDialog;
