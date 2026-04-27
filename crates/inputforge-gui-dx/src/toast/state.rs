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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub level: ToastLevel,
    pub message: String,
    /// Dedupe coalesce count — starts at 1; `push` of an exact duplicate
    /// against the *latest visible* entry (same level + message) increments
    /// this. Older same-content entries are never eligible because newer
    /// toasts of different content have rendered on top of them.
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
        let now = Instant::now();
        let msg = message.into();

        // GC entries that are no longer visible (dismissed OR time-expired —
        // `is_expired` covers both). Without this, a stale entry that's gone
        // from the screen still sits in the Vec and would be resurrected by a
        // matching push below, reappearing with ×2 at its original position
        // (above newer toasts pushed in the meantime).
        self.toasts.retain(|t| !is_expired(t, now));

        // Coalesce — only with the *last* entry, and only when it matches.
        // Older same-content entries are never eligible: a newer toast of
        // different content has rendered on top of them, and incrementing
        // the older one would re-order the visual stack.
        if let Some(last) = self.toasts.last_mut() {
            if last.level == level && last.message == msg {
                last.count = last.count.saturating_add(1);
                last.created = now;
                last.paused = None;
                last.paused_total = Duration::ZERO;
                return;
            }
        }

        // Cap — FIFO drain when exceeded. Counts only non-dismissed entries
        // (post-GC, the only entries left are visible ones, but a prior cap
        // drain may have left a dismissed entry from the same tick).
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
            created: now,
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
#[must_use]
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
    fn push_does_not_coalesce_with_expired_entry() {
        // After a toast has timed out and disappeared from the screen, a
        // subsequent push of the same (level, message) must produce a fresh
        // entry — not resurrect the old one with `count = 2`.
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "tick");
        let first_id = s.toasts[0].id;
        // Backdate the entry past TOAST_DURATION so it's time-expired.
        s.toasts[0].created -= TOAST_DURATION + Duration::from_millis(1);

        s.push(ToastLevel::Info, "tick");

        // Old entry was GC'd; only the fresh one remains, count = 1, new id.
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(s.toasts[0].count, 1);
        assert_ne!(
            s.toasts[0].id, first_id,
            "must be a fresh entry, not a resurrected one"
        );
    }

    #[test]
    fn push_only_coalesces_with_latest_entry() {
        // Newer toast of different content sits between two same-content
        // pushes — the third push must NOT coalesce with the first; it
        // appends a fresh entry at the end so the user sees the repeat
        // below the newest unrelated toast.
        let mut s = ToastState::default();
        s.push(ToastLevel::Info, "alpha");
        s.push(ToastLevel::Info, "beta");
        s.push(ToastLevel::Info, "alpha");

        assert_eq!(s.toasts.len(), 3);
        assert_eq!(s.toasts[0].message, "alpha");
        assert_eq!(s.toasts[0].count, 1);
        assert_eq!(s.toasts[1].message, "beta");
        assert_eq!(s.toasts[1].count, 1);
        assert_eq!(s.toasts[2].message, "alpha");
        assert_eq!(s.toasts[2].count, 1);
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
