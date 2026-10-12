//! One routing policy for current engine state, shared by every entry point.
use inputforge_core::state::{AppState, EngineStatus};
use inputforge_gui_dx::routing::{Action, View};

/// Prepare available routing actions from current engine state.
#[must_use]
pub fn view(state: &AppState) -> View {
    let active = matches!(
        state.engine_status,
        EngineStatus::Running | EngineStatus::Starting
    );
    let pending = !active
        && state
            .active_profile
            .as_ref()
            .and_then(|profile| profile.controllers())
            .is_some_and(|config| {
                state
                    .session
                    .requires_output_reconfiguration(&config.virtual_devices)
            });
    if state.session.offline {
        return View {
            pending,
            ..View::default()
        };
    }
    let faulted = state.engine_status == EngineStatus::Faulted;
    let start = (state.active_profile.is_some() && !active && !pending).then_some(if faulted {
        Action::Retry
    } else {
        Action::Start
    });
    let stop = (state.engine_status != EngineStatus::Stopped).then_some(Action::Stop);
    let toggle = if active {
        stop
    } else if pending {
        Some(Action::ReviewDevices)
    } else {
        start
    };
    let recovery = if pending {
        Some(Action::ReviewDevices)
    } else if state.engine_status == EngineStatus::Running {
        Some(Action::Refresh)
    } else if state.engine_status == EngineStatus::Starting {
        stop
    } else {
        start.map(|action| {
            if faulted || state.session.error.is_some() || !state.session.output_failures.is_empty()
            {
                Action::Retry
            } else {
                action
            }
        })
    };
    View {
        pending,
        toggle,
        start,
        stop,
        recovery,
        refresh: Some(Action::Refresh),
    }
}

/// Revalidate a requested action against current engine state.
#[must_use]
pub fn resolve(state: &AppState, action: Action) -> Option<Action> {
    let view = view(state);
    // A previously rendered Start must never bypass a newly pending layout.
    if view.pending && matches!(action, Action::Start | Action::Retry) {
        return view.toggle;
    }
    [
        view.toggle,
        view.start,
        view.stop,
        view.recovery,
        view.refresh,
    ]
    .contains(&Some(action))
    .then_some(action)
}
