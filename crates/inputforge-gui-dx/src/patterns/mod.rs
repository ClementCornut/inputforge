//! Reusable composed-component patterns. F4 ships `DirtyConfirmDialog`; F15
//! adds `DestructiveConfirmDialog`. Later features may add `SaveBeforeLeave`,
//! bulk-delete confirmations, etc.

pub mod destructive_confirm;
pub mod dirty_confirm;
pub mod live_capture;

pub use destructive_confirm::DestructiveConfirmDialog;
pub use dirty_confirm::DirtyConfirmDialog;
