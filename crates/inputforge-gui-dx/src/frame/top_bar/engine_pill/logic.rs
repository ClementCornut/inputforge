//! Visual engine status, independent of routing eligibility and dispatch.
use inputforge_core::state::EngineStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Variant {
    Live,
    Warning,
    Error,
}

pub(crate) fn engine_pill_state(status: EngineStatus) -> (Variant, &'static str) {
    match status {
        EngineStatus::Starting => (Variant::Warning, "Starting"),
        EngineStatus::Faulted => (Variant::Error, "Faulted"),
        EngineStatus::Running => (Variant::Live, "Running"),
        EngineStatus::Stopped => (Variant::Error, "Stopped"),
    }
}
