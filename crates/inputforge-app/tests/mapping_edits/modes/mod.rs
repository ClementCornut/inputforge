use super::support::*;
use inputforge_core::{action::Action, engine::EngineCommand, mode::Modes, profile::Profile};
use inputforge_gui_dx::mode_management::{Operation, Request, Scope, Transition};
fn request(
    state: &std::sync::Arc<parking_lot::RwLock<inputforge_core::state::AppState>>,
    operation: Operation,
) -> Request {
    Request {
        scope: Scope {
            revision: state.read().mode_revision,
            profile: state.read().active_profile.as_ref().map(|p| p.id().clone()),
            editing: "Default".into(),
        },
        operation,
    }
}
mod dispatch;
mod history;
mod policy;
