//! Prepared notification values; lifetime and queue policy belong to the app.

/// Notification styling and announcement urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// One visible notification in chronological order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub level: ToastLevel,
    pub message: String,
    pub count: u32,
}
