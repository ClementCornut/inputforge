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

impl ToastState {
    pub fn push(&mut self, level: ToastLevel, message: impl Into<String>) {
        let msg = message.into();

        // Coalesce — exact (level, message) match against non-dismissed entries.
        if let Some(t) = self
            .toasts
            .iter_mut()
            .find(|t| !t.dismissed && t.level == level && t.message == msg)
        {
            t.count = t.count.saturating_add(1);
            t.created = Instant::now();
            t.paused = None;
            t.paused_total = Duration::ZERO;
            return;
        }

        // Cap — FIFO drain when exceeded. Counts only non-dismissed entries
        // because dismissed-but-still-in-Vec is the steady state during the
        // CSS fade-out window.
        let visible = self.toasts.iter().filter(|t| !t.dismissed).count();
        if visible >= TOAST_MAX_VISIBLE {
            if let Some(oldest) = self
                .toasts
                .iter_mut()
                .filter(|t| !t.dismissed)
                .min_by_key(|t| t.created)
            {
                oldest.dismissed = true;
            }
        }

        // Append. wrapping_add on u64 is fine: id collisions only arise after
        // 18 quintillion pushes against this single ToastState — not realistic.
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.toasts.push(Toast {
            id,
            level,
            message: msg,
            count: 1,
            created: Instant::now(),
            paused: None,
            paused_total: Duration::ZERO,
            dismissed: false,
        });
    }

    pub fn dismiss(&mut self, id: u64) {
        if let Some(t) = self.toasts.iter_mut().find(|t| t.id == id) {
            t.dismissed = true;
        }
    }

    pub fn pause(&mut self, id: u64) {
        if let Some(t) = self
            .toasts
            .iter_mut()
            .find(|t| t.id == id && !t.dismissed && t.paused.is_none())
        {
            t.paused = Some(Instant::now());
        }
    }

    pub fn resume(&mut self, id: u64) {
        if let Some(t) = self.toasts.iter_mut().find(|t| t.id == id && !t.dismissed) {
            if let Some(start) = t.paused.take() {
                t.paused_total = t.paused_total.saturating_add(start.elapsed());
            }
        }
    }
}

/// Compute whether a toast has exceeded `TOAST_DURATION`, excluding paused
/// intervals (both finalized via `paused_total` and any in-progress pause
/// observed via `paused`).
pub fn is_expired(t: &Toast, now: Instant) -> bool {
    if t.dismissed {
        return true;
    }
    let total = now.saturating_duration_since(t.created);
    let current_pause = t
        .paused
        .map_or(Duration::ZERO, |s| now.saturating_duration_since(s));
    let effective = total.saturating_sub(t.paused_total + current_pause);
    effective >= TOAST_DURATION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_appends_when_empty() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "hi");
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(s.toasts[0].message, "hi");
        assert_eq!(s.toasts[0].count, 1);
        assert!(!s.toasts[0].dismissed);
    }

    #[test]
    fn push_coalesces_exact_string_match() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Warning, "HidHide unavailable");
        s.push(ToastLevel::Warning, "HidHide unavailable");
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(s.toasts[0].count, 2);
    }

    #[test]
    fn push_does_not_coalesce_across_levels() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "Saved");
        s.push(ToastLevel::Warning, "Saved");
        assert_eq!(s.toasts.len(), 2);
        assert_eq!(s.toasts[0].count, 1);
        assert_eq!(s.toasts[1].count, 1);
    }

    #[test]
    fn next_id_is_monotonic() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "a");
        s.push(ToastLevel::Info, "b");
        s.push(ToastLevel::Info, "c");
        assert_eq!(s.toasts[0].id, 0);
        assert_eq!(s.toasts[1].id, 1);
        assert_eq!(s.toasts[2].id, 2);
        assert_eq!(s.next_id, 3);
    }

    #[test]
    fn push_resets_timer_on_coalesce() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "tick");
        let first_created = s.toasts[0].created;
        std::thread::sleep(Duration::from_millis(5));
        s.push(ToastLevel::Info, "tick");
        let second_created = s.toasts[0].created;
        assert!(
            second_created > first_created,
            "coalesce must reset created"
        );
    }

    #[test]
    fn push_drops_oldest_when_cap_exceeded() {
        let mut s = ToastState::default();
        for i in 0..TOAST_MAX_VISIBLE {
            s.push(ToastLevel::Info, format!("msg-{i}"));
        }
        // Fifth push fills the cap exactly. No drain yet.
        let visible_now = s.toasts.iter().filter(|t| !t.dismissed).count();
        assert_eq!(visible_now, TOAST_MAX_VISIBLE);

        // Sixth push triggers the drain — the very first toast ("msg-0")
        // is the oldest non-dismissed entry.
        s.push(ToastLevel::Info, "overflow");

        let visible_after = s.toasts.iter().filter(|t| !t.dismissed).count();
        assert_eq!(visible_after, TOAST_MAX_VISIBLE);

        // The Vec carries 6 entries total (5 originally + 1 new). The "msg-0"
        // entry is now dismissed, every other original entry is still live.
        assert_eq!(s.toasts.len(), TOAST_MAX_VISIBLE + 1);
        assert!(s.toasts[0].dismissed, "oldest must be dismissed");
        assert_eq!(s.toasts[0].message, "msg-0");
        for i in 1..TOAST_MAX_VISIBLE {
            assert!(!s.toasts[i].dismissed, "non-oldest must stay live");
        }
        assert_eq!(s.toasts.last().unwrap().message, "overflow");
    }

    #[test]
    fn dismiss_marks_entry_dismissed() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "go");
        let id = s.toasts[0].id;
        s.dismiss(id);
        assert!(s.toasts[0].dismissed);
        // Idempotent — second dismiss is a no-op.
        s.dismiss(id);
        assert!(s.toasts[0].dismissed);
    }

    #[test]
    fn pause_resume_accumulates_paused_total() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "p");
        let id = s.toasts[0].id;
        s.pause(id);
        std::thread::sleep(Duration::from_millis(8));
        s.resume(id);
        let after_first = s.toasts[0].paused_total;
        assert!(after_first >= Duration::from_millis(7));
        s.pause(id);
        std::thread::sleep(Duration::from_millis(5));
        s.resume(id);
        let after_second = s.toasts[0].paused_total;
        assert!(after_second > after_first, "second resume must accumulate");
    }

    #[test]
    fn is_expired_excludes_paused_time() {
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "x");
        let toast = s.toasts[0].clone();
        let now = toast.created + TOAST_DURATION + Duration::from_millis(1);
        // No pauses → expired right at TOAST_DURATION.
        assert!(is_expired(&toast, now));

        // Paused for the entire elapsed window → effective elapsed is zero.
        let mut t2 = toast.clone();
        t2.paused = Some(t2.created);
        assert!(!is_expired(&t2, now));
    }
}
