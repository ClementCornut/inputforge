// Rust guideline compliant 2026-05-03

//! Shared keyboard-nudge undo coalesce: same-`(stage_id, key)` repeats
//! arriving within `COALESCE_WINDOW_MS` merge into the prior undo entry.
//! Embedded as a field on each editor's `BodyState`. Generic over the
//! editor-specific `KeyKind` so `instruments/` carries no back-import to F10
//! or F11; each editor instantiates `NudgeCoalesce<KeyKind>` with its own
//! local enum.

const COALESCE_WINDOW_MS: u64 = 250;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NudgeCoalesce<K: Copy + Eq> {
    last_at_ms: Option<u64>,
    last_key: Option<K>,
}

// Manual `Default` (not derived) so the impl does not require `K: Default`.
// `KeyKind` (F10) and the future F11 `KeyKind` are bare enums with no
// natural default, so the derive's automatic `K: Default` bound would block
// instantiation. `Option::None` is the meaningful "no prior nudge" state.
impl<K: Copy + Eq> Default for NudgeCoalesce<K> {
    fn default() -> Self {
        Self {
            last_at_ms: None,
            last_key: None,
        }
    }
}

impl<K: Copy + Eq> NudgeCoalesce<K> {
    /// Decide whether a nudge at `now_ms` for `key` should merge into the
    /// previously-recorded entry (true) or push a new undo entry (false).
    /// The caller invokes `record` after dispatching to persist the new
    /// timestamp/key for the next merge decision.
    pub(crate) fn should_merge(&self, now_ms: u64, key: K) -> bool {
        match (self.last_at_ms, self.last_key) {
            (Some(prev), Some(prev_key)) => {
                prev_key == key && now_ms.saturating_sub(prev) <= COALESCE_WINDOW_MS
            }
            _ => false,
        }
    }

    pub(crate) fn record(&mut self, now_ms: u64, key: K) {
        self.last_at_ms = Some(now_ms);
        self.last_key = Some(key);
    }

    pub(crate) fn reset(&mut self) {
        self.last_at_ms = None;
        self.last_key = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::keyboard::KeyKind;

    #[test]
    fn no_prior_nudge_returns_false() {
        let coalesce = NudgeCoalesce::default();
        assert!(!coalesce.should_merge(0, KeyKind::ArrowLeft));
    }

    #[test]
    fn same_key_within_window_returns_true() {
        let mut coalesce = NudgeCoalesce::default();
        coalesce.record(100, KeyKind::ArrowRight);
        assert!(coalesce.should_merge(200, KeyKind::ArrowRight));
    }

    #[test]
    fn same_key_past_window_returns_false() {
        let mut coalesce = NudgeCoalesce::default();
        coalesce.record(100, KeyKind::ArrowRight);
        assert!(!coalesce.should_merge(100 + 251, KeyKind::ArrowRight));
    }

    #[test]
    fn different_key_within_window_returns_false() {
        let mut coalesce = NudgeCoalesce::default();
        coalesce.record(100, KeyKind::ArrowRight);
        assert!(!coalesce.should_merge(150, KeyKind::ArrowLeft));
    }

    #[test]
    fn reset_clears_prior_state() {
        let mut coalesce = NudgeCoalesce::default();
        coalesce.record(100, KeyKind::ArrowRight);
        coalesce.reset();
        assert!(!coalesce.should_merge(150, KeyKind::ArrowRight));
    }
}
