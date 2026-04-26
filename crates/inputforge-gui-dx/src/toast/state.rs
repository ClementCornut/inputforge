//! Pure-data layer for the toast queue. No Dioxus runtime dependency —
//! every method on `ToastState` is `&mut self` and the unit tests construct
//! `ToastState::default()` directly. The Signal wrapper lives in `queue.rs`.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: u64,
    pub level: ToastLevel,
    pub message: String,
    /// Dedupe coalesce count — starts at 1; `push` of an exact duplicate
    /// against a non-dismissed entry increments this.
    pub count: u32,
    pub created: Instant,
    /// Some(start) while the toast is hover/focus-paused.
    pub paused: Option<Instant>,
    /// Accumulated pause time across resume cycles.
    pub paused_total: Duration,
    pub dismissed: bool,
}

#[derive(Debug, Default)]
pub struct ToastState {
    pub toasts: Vec<Toast>,
    pub next_id: u64,
}

/// Auto-dismiss duration excluding paused intervals.
pub const TOAST_DURATION: Duration = Duration::from_secs(8);

/// Cap on simultaneously-visible (non-dismissed) toasts. Push beyond the cap
/// FIFO-drains the oldest non-dismissed entry.
pub const TOAST_MAX_VISIBLE: usize = 5;
