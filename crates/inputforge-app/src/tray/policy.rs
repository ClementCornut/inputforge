//! Routing state and tray labels use the engine's current state.
use inputforge_core::state::EngineStatus;

pub(crate) fn controller_changes_pending(state: &inputforge_core::state::AppState) -> bool {
    !matches!(
        state.engine_status,
        EngineStatus::Running | EngineStatus::Starting
    ) && state
        .active_profile
        .as_ref()
        .and_then(|profile| profile.controllers())
        .is_some_and(|config| {
            state
                .session
                .requires_output_reconfiguration(&config.virtual_devices)
        })
}

/// Label for the tray's toggle menu item, derived from current engine status.
///
/// Matches the application routing command policy.
pub(crate) fn toggle_label(status: EngineStatus, pending: bool) -> &'static str {
    if pending {
        return "Review controller changes";
    }
    match status {
        EngineStatus::Running | EngineStatus::Starting => "Stop",
        EngineStatus::Stopped | EngineStatus::Faulted => "Start",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_controller_review_uses_live_state() {
        use inputforge_core::{
            mode::Modes,
            profile::{Profile, controllers::ControllerConfig},
            state::AppState,
            types::VirtualDeviceConfig,
        };
        let mut state = AppState::new();
        assert!(!controller_changes_pending(&state));
        let mut profile = Profile::new(
            "tray".into(),
            vec![],
            Modes::new(vec!["Default".into()]).unwrap(),
            vec![],
            vec![],
            "Default".into(),
        );
        profile
            .set_controllers(ControllerConfig::default())
            .unwrap();
        state.active_profile = Some(profile);
        state.session.output_active = true;
        state.session.output_layout = vec![VirtualDeviceConfig {
            device_id: 1,
            axes: vec![],
            button_count: 2,
            hat_count: 0,
        }];
        for status in [EngineStatus::Stopped, EngineStatus::Faulted] {
            state.engine_status = status;
            assert!(controller_changes_pending(&state));
        }
        for status in [EngineStatus::Starting, EngineStatus::Running] {
            state.engine_status = status;
            assert!(!controller_changes_pending(&state));
        }
        state.engine_status = EngineStatus::Stopped;
        state.session.output_active = false;
        assert!(!controller_changes_pending(&state));
    }

    #[test]
    fn toggle_label_matches_dispatch_mapping() {
        assert_eq!(toggle_label(EngineStatus::Running, false), "Stop");
        assert_eq!(toggle_label(EngineStatus::Stopped, false), "Start");
        assert_eq!(
            toggle_label(EngineStatus::Stopped, true),
            "Review controller changes"
        );
    }
}
