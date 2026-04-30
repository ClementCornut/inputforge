// Rust guideline compliant 2026-04-29

//! Pure dispatch logic for the engine pill.

use inputforge_core::engine::EngineCommand;
use inputforge_core::state::EngineStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Variant {
    Live,
    Warning,
    Error,
}

impl Variant {
    pub(crate) fn class_suffix(self) -> &'static str {
        match self {
            Variant::Live => "live",
            Variant::Warning => "warning",
            Variant::Error => "error",
        }
    }
}

/// Map an engine status to (visual variant, label, command-on-click).
///
/// The caller is responsible for rendering `disabled: !has_profile` on
/// the button, which makes the returned command unreachable in the
/// no-profile case. The dispatch table is therefore a pure function of
/// `status` only — no advisory profile flag needed.
pub(crate) fn engine_pill_state(status: EngineStatus) -> (Variant, &'static str, EngineCommand) {
    match status {
        EngineStatus::Running => (Variant::Live, "Running", EngineCommand::Deactivate),
        EngineStatus::Paused => (Variant::Warning, "Paused", EngineCommand::Activate),
        EngineStatus::Stopped => (Variant::Error, "Stopped", EngineCommand::Activate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_yields_live_running_deactivate() {
        let (v, l, cmd) = engine_pill_state(EngineStatus::Running);
        assert_eq!(v, Variant::Live);
        assert_eq!(l, "Running");
        assert!(matches!(cmd, EngineCommand::Deactivate));
    }

    #[test]
    fn paused_yields_warning_paused_activate() {
        let (v, l, cmd) = engine_pill_state(EngineStatus::Paused);
        assert_eq!(v, Variant::Warning);
        assert_eq!(l, "Paused");
        assert!(matches!(cmd, EngineCommand::Activate));
    }

    #[test]
    fn stopped_yields_error_stopped_activate() {
        let (v, l, cmd) = engine_pill_state(EngineStatus::Stopped);
        assert_eq!(v, Variant::Error);
        assert_eq!(l, "Stopped");
        assert!(matches!(cmd, EngineCommand::Activate));
    }
}
